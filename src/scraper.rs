use crate::error::{FetchBaseUrlError, FetchPaperError};
use nonempty::NonEmpty;
use reqwest::{Client, header, redirect};
use scraper::{Html, Selector};
use std::{cmp::Ordering, collections::BinaryHeap};
use url::Url;
use lazy_static::lazy_static;

#[derive(Debug)]
pub struct Scraper {
    client: Client,
    pub base_urls: BinaryHeap<WeightedUrl>,
}

impl Scraper {
    /// Creates a new [`SciHubScraper`] with the given sci-hub base url.
    #[must_use]
    pub fn with_base_url(base_url: Url) -> Self {
        Self::with_base_urls(NonEmpty::singleton(base_url))
    }

    /// Creates a new [`SciHubScraper`] with the given sci-hub base urls.
    #[must_use]
    pub fn with_base_urls(base_urls: NonEmpty<Url>) -> Self {
        Self::with_base_urls_and_client(base_urls, Client::new())
    }

    /// Creates a new [`SciHubScraper`] with the given sci-hub base urls and [`Client`].
    #[must_use]
    pub fn with_base_urls_and_client(base_urls: NonEmpty<Url>, client: Client) -> Self {
        Scraper {
            client,
            base_urls: Self::base_urls_as_heap(base_urls),
        }
    }

    /// Creates a new [`SciHubScraper`] with automatically detected base urls.
    pub async fn with_auto_detected_base_urls() -> Result<Self, FetchBaseUrlError> {
        let client = Client::new();
        let base_urls = fetch_base_urls(&client).await?;
        Ok(Self::with_base_urls_and_client(base_urls, client))
    }

    /// Generates a scihub paper url from the given base url and doi.
    pub fn scihub_url_from_base_url_and_doi(
        base_url: &Url,
        doi: &str,
    ) -> Result<Url, url::ParseError> {
        base_url.join(doi)
    }
    fn convert_protocol_relative_url_to_absolute(relative_url: &str, absolute_url: &Url) -> String {
        if relative_url.starts_with("//") {
            format!("{}:{}", absolute_url.scheme(), relative_url)
        } else {
            relative_url.to_string()
        }
    }
    fn base_urls_as_heap(base_urls: NonEmpty<Url>) -> BinaryHeap<WeightedUrl> {
        let mut heap = BinaryHeap::with_capacity(base_urls.len());
        for base_url in base_urls {
            heap.push(base_url.into());
        }
        heap
    }

    /// Fetches the paper with the given doi from sci-hub.
    pub async fn fetch_paper_by_doi(&mut self, doi: &str) -> Result<Paper, FetchPaperError> {
        let mut failing_urls: Vec<WeightedUrl> = Vec::new();
        let mut last_error = None;
        while let Some(base_url) = self.base_urls.peek() {
            let url = Self::scihub_url_from_base_url_and_doi(&base_url.url, doi)?;

            match self.fetch_paper_from_scihub_url(url).await {
                Ok(paper) => {
                    for mut failing_url in failing_urls {
                        failing_url.weight -= 10;
                        self.base_urls.push(failing_url);
                    }
                    let mut working_base_url = self.base_urls.peek_mut().unwrap();
                    working_base_url.weight += 1;
                    return Ok(paper);
                }
                Err(err) => {
                    failing_urls.push(self.base_urls.pop().unwrap());
                    last_error = Some(err);
                }
            };
        }
        Err(last_error.unwrap())
    }
    /// Fetches the paper with the given url from sci-hub, automatically fetching current sci-hub domains.
    pub async fn fetch_paper_by_paper_url(&mut self, url: &str) -> Result<Paper, FetchPaperError> {
        self.fetch_paper_by_doi(url).await
    }
    /// Fetches the paper with the given doi using the given sci-hub base url.
    pub async fn fetch_paper_by_base_url_and_doi(
        &self,
        base_url: &Url,
        doi: &str,
    ) -> Result<Paper, FetchPaperError> {
        let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;
        self.fetch_paper_from_scihub_url(url).await
    }
    /// Fetches the paper from the given scihub url.
    pub async fn fetch_paper_from_scihub_url(&self, url: Url) -> Result<Paper, FetchPaperError> {
        let document = fetch_html_document(&self.client, url.clone()).await?;

        lazy_static! {
            static ref TITLE_SELECTOR: Selector = Selector::parse("head title").unwrap();
            static ref DOWNLOAD_BUTTON_SELECTOR: Selector =
                Selector::parse("#buttons [onclick]").unwrap();
            static ref VERSIONS_SELECTOR: Selector = Selector::parse("#versions a[href]").unwrap();
            static ref BOLD_SELECTOR: Selector = Selector::parse("b").unwrap();
        }

        let (doi, paper_title) = document
            .select(&TITLE_SELECTOR)
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
            .select(&DOWNLOAD_BUTTON_SELECTOR)
            .filter_map(|node| node.value().attr("onclick"))
            .filter_map(|attrval| Some(&attrval[attrval.find('\'')? + 1..attrval.rfind('\'')?]))
            .next()
            .ok_or(FetchPaperError::MissingPdfUrl)?;
        let pdf_url = Self::convert_protocol_relative_url_to_absolute(raw_pdf_url, &url);

        let mut current_version = None;
        let other_versions: Vec<_> = document
            .select(&VERSIONS_SELECTOR)
            .filter_map(|node| {
                if current_version.is_none()
                    && let Some(version_str) =
                        node.select(&BOLD_SELECTOR).next().map(|b| b.inner_html())
                {
                    current_version = Some(version_str);
                    return None; // do not include current version
                }

                let version_href = node.value().attr("href")?;
                let version_url =
                    Self::convert_protocol_relative_url_to_absolute(version_href, &url);

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

    /// Fetches the pdf url of the paper with the given doi from sci-hub, automatically fetching current sci-hub domains.
    pub async fn fetch_paper_pdf_url_by_doi(&mut self, doi: &str) -> Result<Url, FetchPaperError> {
        let mut failing_urls: Vec<WeightedUrl> = Vec::new();
        let mut last_error = None;
        while let Some(base_url) = self.base_urls.peek() {
            let url = Self::scihub_url_from_base_url_and_doi(&base_url.url, doi)?;

            match self.fetch_paper_pdf_url_from_scihub_url(url).await {
                Ok(paper) => {
                    for mut failing_url in failing_urls {
                        failing_url.weight -= 10;
                        self.base_urls.push(failing_url);
                    }
                    let mut working_base_url = self.base_urls.peek_mut().unwrap();
                    working_base_url.weight += 1;
                    return Ok(paper);
                }
                Err(err) => {
                    failing_urls.push(self.base_urls.pop().unwrap());
                    last_error = Some(err);
                }
            };
        }
        Err(last_error.unwrap())
    }
    /// Fetches the pdf url of the paper with the given url from sci-hub, automatically fetching current sci-hub domains.
    pub async fn fetch_paper_pdf_url_by_paper_url(
        &mut self,
        url: &str,
    ) -> Result<Url, FetchPaperError> {
        self.fetch_paper_pdf_url_by_doi(url).await
    }
    /// Fetches the pdf url of the paper with the given doi using the given sci-hub base url.
    pub async fn fetch_paper_pdf_url_by_base_url_and_doi(
        &self,
        base_url: &Url,
        doi: &str,
    ) -> Result<Url, FetchPaperError> {
        let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;
        self.fetch_paper_pdf_url_from_scihub_url(url).await
    }
    /// Fetches the pdf url of the paper from the given scihub url.
    pub async fn fetch_paper_pdf_url_from_scihub_url(
        &self,
        url: Url,
    ) -> Result<Url, FetchPaperError> {
        let client = Client::builder()
            .redirect(redirect::Policy::none())
            .build()?;

        let response = client
            .get(url.clone())
            .header(
                header::USER_AGENT,
                "Mozilla/5.0 (Android 4.4; Mobile; rv:42.0) Gecko/42.0 Firefox/42.0",
            ) // "disguise" as mobile (mobile page allows easier scraping)
            .send()
            .await?;
        dbg!(&url);
        dbg!(&response);

        response
            .headers()
            .get(header::LOCATION)
            .ok_or(FetchPaperError::MissingPdfUrl)?
            .to_str()
            .map_err(|_| FetchPaperError::MalformedPdfUrl)
            .map(|pdf_url| Self::convert_protocol_relative_url_to_absolute(pdf_url, &url))
            .and_then(|url_str| Url::parse(&url_str).map_err(|e| e.into()))
            .and_then(|url| {
                if url.domain().is_some_and(|e| e.contains("sci-hub")) {
                    Ok(url)
                } else {
                    Err(FetchPaperError::InvalidRedirect)
                }
            })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Paper {
    pub scihub_url: Url,
    pub doi: String,
    pub title: String,
    pub version: String,
    pub download_url: Url,
    // pub citation: String,
    pub other_versions: Vec<PaperVersion>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PaperVersion {
    pub version: String,
    pub scihub_url: Url,
}

#[derive(Debug, Clone)]
pub struct WeightedUrl {
    pub url: Url,
    weight: i32,
}
impl PartialEq for WeightedUrl {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for WeightedUrl {}
impl PartialOrd for WeightedUrl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for WeightedUrl {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}
impl From<Url> for WeightedUrl {
    fn from(url: Url) -> Self {
        WeightedUrl { url, weight: 0 }
    }
}
impl From<WeightedUrl> for Url {
    fn from(url: WeightedUrl) -> Url {
        url.url
    }
}

/// Fetches a list of base urls from wikipedia and using brave search.
pub async fn fetch_base_urls(client: &Client) -> Result<NonEmpty<Url>, FetchBaseUrlError> {
    let provider_urls = [
        "https://en.wikipedia.org/wiki/Sci-Hub",
        "https://search.brave.com/search?q=sci-hub",
    ];

    let mut base_urls = Vec::new();
    for provider_url in provider_urls {
        let provider_url = Url::parse(provider_url).unwrap();
        match fetch_base_urls_from_provider(client, provider_url).await {
            Ok(mut urls) => {
                base_urls.push(urls.head);
                base_urls.append(&mut urls.tail)
            }
            Err(FetchBaseUrlError::NoneFound) => continue,
            Err(err) => return Err(err),
        };
    }

    if base_urls.is_empty() {
        return Err(FetchBaseUrlError::NoneFound);
    }

    if let Some(last) = base_urls.pop() {
        Ok(NonEmpty {
            head: last,
            tail: base_urls,
        })
    } else {
        Err(FetchBaseUrlError::NoneFound)
    }
}

/// Fetches a list of base urls from the given provider.
pub async fn fetch_base_urls_from_provider(
    client: &Client,
    provider_url: Url,
) -> Result<NonEmpty<Url>, FetchBaseUrlError> {
    let document = fetch_html_document(client, provider_url).await?;

    lazy_static! {
        static ref LINK_SELECTOR: Selector = Selector::parse("a[href]").unwrap();
    }

    let mut base_urls: Vec<Url> = document
        .select(&LINK_SELECTOR)
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
                .is_some_and(|host| host.contains("sci-hub") || host.contains("scihub"))
        })
        .collect();

    base_urls.sort_unstable();
    base_urls.dedup();

    if let Some(last) = base_urls.pop() {
        Ok(NonEmpty {
            head: last,
            tail: base_urls,
        })
    } else {
        Err(FetchBaseUrlError::NoneFound)
    }
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
