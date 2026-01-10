use crate::{
    error::{FetchBaseUrlError, FetchPaperError},
    url_pool::UrlPool,
};
use nonempty::NonEmpty;
use reqwest::{Client, header};
use scraper::{Html, Selector};
use std::sync::OnceLock;
use url::Url;

fn title_selector() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("head title").unwrap())
}
fn download_selector() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("#buttons [onclick]").unwrap())
}
fn versions_selector() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("#versions a[href]").unwrap())
}
fn bold_selector() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("b").unwrap())
}
fn link_selector() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("a[href]").unwrap())
}

#[derive(Debug)]
pub struct Scraper {
    client: Client,
    pool: UrlPool,
}

impl Scraper {
    /// Creates a new [`Scraper`] with the given scihub base url.
    #[must_use]
    pub fn with_base_url(base_url: Url) -> Self {
        Self::with_base_urls(NonEmpty::singleton(base_url))
    }

    /// Creates a new [`Scraper`] with the given scihub base urls.
    #[must_use]
    pub fn with_base_urls(base_urls: NonEmpty<Url>) -> Self {
        Self::with_base_urls_and_client(base_urls, Client::new())
    }

    /// Creates a new [`Scraper`] with the given scihub base urls and [`Client`].
    #[must_use]
    pub fn with_base_urls_and_client(base_urls: NonEmpty<Url>, client: Client) -> Self {
        Scraper {
            client,
            pool: UrlPool::new(base_urls),
        }
    }

    /// Creates a new [`Scraper`] with automatically detected base urls.
    pub async fn with_auto_detected_base_urls() -> Result<Self, FetchBaseUrlError> {
        let client = Client::new();
        let base_urls = fetch_base_urls(&client).await?;
        Ok(Self::with_base_urls_and_client(base_urls, client))
    }

    /// Fetches the paper with the given doi from scihub.
    pub async fn fetch_paper_by_doi(&mut self, doi: &str) -> Result<Paper, FetchPaperError> {
        let mut failing_urls = Vec::new();
        let mut last_error = None;

        while let Some(weighted) = self.pool.pop() {
            let url = weighted.url.join(doi)?;

            match self.fetch_paper_from_scihub_url(url).await {
                Ok(paper) => {
                    self.pool.report_success(weighted, failing_urls);
                    return Ok(paper);
                }
                Err(err) => {
                    failing_urls.push(weighted);
                    last_error = Some(err);
                }
            }
        }

        self.pool.report_failures(failing_urls);
        Err(last_error.unwrap_or(FetchPaperError::MissingPaperInfo))
    }

    /// Fetches the paper from the given scihub url.
    pub async fn fetch_paper_from_scihub_url(&self, url: Url) -> Result<Paper, FetchPaperError> {
        let document = fetch_html_document(&self.client, url.clone()).await?;

        let (doi, paper_title) = document
            .select(title_selector())
            .find_map(|node| {
                let title = node.inner_html();
                let mut iter = title.rsplit('|').map(str::trim);
                match (iter.next(), iter.next()) {
                    (Some(doi), Some(page_title)) => {
                        Some((doi.to_string(), page_title.to_string()))
                    }
                    _ => None,
                }
            })
            .ok_or(FetchPaperError::MissingPaperInfo)?;

        let raw_pdf_url = document
            .select(download_selector())
            .filter_map(|node| node.value().attr("onclick"))
            .filter_map(|attrval| {
                let start = attrval.find('\'')? + 1;
                let end = attrval.rfind('\'')?;
                Some(&attrval[start..end])
            })
            .next()
            .ok_or(FetchPaperError::MissingPdfUrl)?;

        let pdf_url = self.convert_protocol_relative_url_to_absolute(raw_pdf_url, &url);

        let mut current_version = None;
        let other_versions: Vec<_> = document
            .select(versions_selector())
            .filter_map(|node| {
                if current_version.is_none()
                    && let Some(version_str) =
                        node.select(bold_selector()).next().map(|b| b.inner_html())
                {
                    current_version = Some(version_str);
                    return None;
                }

                let version_href = node.value().attr("href")?;
                let version_url =
                    self.convert_protocol_relative_url_to_absolute(version_href, &url);

                Some(PaperVersion {
                    version: node.inner_html(),
                    scihub_url: Url::parse(&version_url).ok()?,
                })
            })
            .collect();

        let current_version = current_version.unwrap_or_else(|| "current".to_string());

        Ok(Paper {
            scihub_url: url,
            doi,
            title: paper_title,
            version: current_version,
            download_url: Url::parse(&pdf_url)?,
            other_versions,
        })
    }

    // TODO: readd method for directly fetching the pdf url
    // for some mirrors we can request the mobile page to directly get the pdf
    // for other mirros we can request [base_url]/pdf/[doi].pdf
    // for other none of these work

    fn convert_protocol_relative_url_to_absolute(
        &self,
        relative_url: &str,
        absolute_url: &Url,
    ) -> String {
        if relative_url.starts_with("//") {
            format!("{}:{}", absolute_url.scheme(), relative_url)
        } else {
            relative_url.to_string()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Paper {
    pub scihub_url: Url,
    pub doi: String,
    pub title: String,
    pub version: String,
    pub download_url: Url,
    pub other_versions: Vec<PaperVersion>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PaperVersion {
    pub version: String,
    pub scihub_url: Url,
}

/// Fetches a list of base urls from wikipedia and using brave search.
pub async fn fetch_base_urls(client: &Client) -> Result<NonEmpty<Url>, FetchBaseUrlError> {
    let provider_urls = [
        "https://en.wikipedia.org/wiki/scihub",
        "https://search.brave.com/search?q=scihub",
    ];

    let mut base_urls = Vec::new();
    for provider_url in provider_urls {
        let provider_url = Url::parse(provider_url).unwrap();
        if let Ok(urls) = fetch_base_urls_from_provider(client, provider_url).await {
            base_urls.extend(urls);
        }
    }

    base_urls.sort_unstable();
    base_urls.dedup();

    NonEmpty::from_vec(base_urls).ok_or(FetchBaseUrlError::NoneFound)
}

/// Fetches a list of base urls from the given provider.
pub async fn fetch_base_urls_from_provider(
    client: &Client,
    provider_url: Url,
) -> Result<NonEmpty<Url>, FetchBaseUrlError> {
    let document = fetch_html_document(client, provider_url).await?;

    let urls: Vec<Url> = document
        .select(link_selector())
        .filter_map(|node| node.value().attr("href"))
        .filter_map(|href| Url::parse(href).ok())
        .map(|mut url| {
            url.set_path("");
            url.set_query(None);
            url.set_fragment(None);
            url
        })
        .filter(|url| {
            url.host_str()
                .is_some_and(|host| host.contains("scihub") || host.contains("sci-hub"))
        })
        .collect();

    NonEmpty::from_vec(urls).ok_or(FetchBaseUrlError::NoneFound)
}

async fn fetch_html_document(client: &Client, url: Url) -> Result<Html, reqwest::Error> {
    let text = client
        .get(url)
        .header(header::ACCEPT, "text/html")
        .send()
        .await?
        .text()
        .await?;
    Ok(Html::parse_document(&text))
}
