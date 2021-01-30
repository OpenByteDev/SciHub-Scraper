use crate::error::Error;
use reqwest::{header, redirect, Client};
use scraper::{Html, Selector};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use url::Url;

pub struct SciHubScraper {
    client: Client,
    pub base_urls: BinaryHeap<WeightedUrl>,
}

impl Default for SciHubScraper {
    fn default() -> Self {
        Self::new()
    }
}

impl SciHubScraper {
    #[must_use]
    pub fn new() -> Self {
        SciHubScraper {
            client: Client::new(),
            base_urls: BinaryHeap::new(),
        }
    }
    /// Creates a new `SciHubScraper` with the given sci-hub base url. (This will disable the automatic sci-hub domain detection).
    #[must_use]
    pub fn with_base_url(base_url: Url) -> Self {
        Self::with_base_urls(vec![base_url])
    }
    /// Creates a new `SciHubScraper` with the given sci-hub base urls. (This will disable the automatic sci-hub domain detection).
    #[must_use]
    pub fn with_base_urls(base_urls: Vec<Url>) -> Self {
        SciHubScraper {
            client: Client::new(),
            base_urls: Self::base_urls_as_heap(base_urls),
        }
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
    fn base_urls_as_heap(base_urls: Vec<Url>) -> BinaryHeap<WeightedUrl> {
        let mut heap = BinaryHeap::with_capacity(base_urls.len());
        for base_url in base_urls {
            heap.push(base_url.into());
        }
        heap
    }

    /// Fetches a list of base urls from sci-hub.now.sh and adds them to the base url heap.
    pub async fn fetch_base_urls(&mut self) -> Result<&BinaryHeap<WeightedUrl>, Error> {
        let scihub_now_url = Url::parse("https://sci-hub.now.sh/").unwrap();
        self.fetch_base_urls_from_provider(scihub_now_url).await
    }
    /// Fetches a list of base urls from the given provider and adds them to the base url heap.
    pub async fn fetch_base_urls_from_provider(
        &mut self,
        scihub_url_provider: Url,
    ) -> Result<&BinaryHeap<WeightedUrl>, Error> {
        let document = self.fetch_html_document(scihub_url_provider).await?;

        lazy_static! {
            static ref LINK_SELECTOR: Selector = Selector::parse("a[href]").unwrap();
        }

        let mut base_urls: Vec<Url> = document
            .select(&LINK_SELECTOR)
            .filter_map(|node| node.value().attr("href"))
            .filter_map(|href| Url::parse(href).ok())
            .filter(|url| {
                url.domain().map_or(false, |e| {
                    e.starts_with("sci-hub") && !e.ends_with("now.sh")
                })
            })
            .collect();
        base_urls.dedup();

        self.base_urls.reserve(base_urls.len());
        for base_url in base_urls {
            self.base_urls.push(base_url.into());
        }

        Ok(&self.base_urls)
    }
    /// Ensures a list of base urls by fetching them from the default provider if there are none currently.
    pub async fn ensure_base_urls(&mut self) -> Result<&BinaryHeap<WeightedUrl>, Error> {
        if self.base_urls.is_empty() {
            self.fetch_base_urls().await?;
            if self.base_urls.is_empty() {
                return Err(Error::Other("Failed to load sci-hub base urls."));
            }
        }
        Ok(&self.base_urls)
    }

    /*async fn try_fetch_with_base_urls<T, F:Future<Output=Result<T, Error>>, FN: Fn(Url) -> F>(&mut self, doi: &str, fetch_fn: FN) -> Result<T, Error> {
        self.ensure_base_urls().await?;

        let mut failing_urls: Vec<WeightedUrl> = Vec::new();
        while !self.base_urls.is_empty() {
            let base_url = &self.base_urls.peek().unwrap().url; // we are guaranteed to have at least one base url.
            let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;

            let result = self.fetch_paper_from_scihub_url(url).await;

            if result.is_ok() {
                for mut failing_url in failing_urls {
                    failing_url.weight -= 10;
                    self.base_urls.push(failing_url);
                }
                let mut working_base_url = self.base_urls.peek_mut().unwrap();
                working_base_url.weight += 1;
                return result;
            } else {
                failing_urls.push(self.base_urls.pop().unwrap())
            }
        }

        Err(Error::Other("Invalid doi or no working sci-hub mirror found"))
    }*/

    /// Fetches the paper with the given doi from sci-hub, automatically fetching current sci-hub domains.
    pub async fn fetch_paper_by_doi(&mut self, doi: &str) -> Result<Paper, Error> {
        self.ensure_base_urls().await?;

        let mut failing_urls: Vec<WeightedUrl> = Vec::new();
        while !self.base_urls.is_empty() {
            let base_url = &self.base_urls.peek().unwrap().url; // we are guaranteed to have at least one base url.
            let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;

            let result = self.fetch_paper_from_scihub_url(url).await;

            if result.is_ok() {
                for mut failing_url in failing_urls {
                    failing_url.weight -= 10;
                    self.base_urls.push(failing_url);
                }
                let mut working_base_url = self.base_urls.peek_mut().unwrap();
                working_base_url.weight += 1;
                return result;
            } else {
                failing_urls.push(self.base_urls.pop().unwrap())
            }
        }

        Err(Error::Other(
            "Invalid doi or no working sci-hub mirror found",
        ))
    }
    /// Fetches the paper with the given url from sci-hub, automatically fetching current sci-hub domains.
    pub async fn fetch_paper_by_paper_url(&mut self, url: &str) -> Result<Paper, Error> {
        self.fetch_paper_by_doi(url).await
    }
    /// Fetches the paper with the given doi using the given sci-hub base url.
    pub async fn fetch_paper_by_base_url_and_doi(
        &self,
        base_url: &Url,
        doi: &str,
    ) -> Result<Paper, Error> {
        let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;
        self.fetch_paper_from_scihub_url(url).await
    }
    /// Fetches the paper from the given scihub url.
    pub async fn fetch_paper_from_scihub_url(&self, url: Url) -> Result<Paper, Error> {
        let document = self.fetch_html_document(url.clone()).await?;

        lazy_static! {
            static ref TITLE_SELECTOR: Selector = Selector::parse("head title").unwrap();
            static ref DOWNLOAD_BUTTON_SELECTOR: Selector =
                Selector::parse("#buttons a[onclick]").unwrap();
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
            .ok_or(Error::SciHubParse("Paper info not found in page."))?;

        let raw_pdf_url = document
            .select(&DOWNLOAD_BUTTON_SELECTOR)
            .filter_map(|node| node.value().attr("onclick"))
            .filter_map(|attrval| Some(&attrval[attrval.find('\'')? + 1..attrval.rfind('\'')?]))
            .next()
            .ok_or(Error::SciHubParse("Pdf url not found in page."))?;
        let pdf_url = Self::convert_protocol_relative_url_to_absolute(raw_pdf_url, &url);

        let mut current_version = None;
        let other_versions: Vec<_> = document
            .select(&VERSIONS_SELECTOR)
            .filter_map(|node| {
                if current_version.is_none() {
                    if let Some(version_str) =
                        node.select(&BOLD_SELECTOR).next().map(|b| b.inner_html())
                    {
                        current_version = Some(version_str);
                        return None; // do not include current version
                    }
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
    pub async fn fetch_paper_pdf_url_by_doi(&mut self, doi: &str) -> Result<Url, Error> {
        self.ensure_base_urls().await?;

        let mut failing_urls: Vec<WeightedUrl> = Vec::new();
        while !self.base_urls.is_empty() {
            let base_url = &self.base_urls.peek().unwrap().url; // we are guaranteed to have at least one base url.
            let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;

            let result = self.fetch_paper_pdf_url_from_scihub_url(url).await;

            if result.is_ok() {
                for mut failing_url in failing_urls {
                    failing_url.weight -= 10;
                    self.base_urls.push(failing_url);
                }
                let mut working_base_url = self.base_urls.peek_mut().unwrap();
                working_base_url.weight += 1;
                return result;
            } else {
                failing_urls.push(self.base_urls.pop().unwrap())
            }
        }

        Err(Error::Other(
            "Invalid doi or no working sci-hub mirror found",
        ))
    }
    /// Fetches the pdf url of the paper with the given url from sci-hub, automatically fetching current sci-hub domains.
    pub async fn fetch_paper_pdf_url_by_paper_url(&mut self, url: &str) -> Result<Url, Error> {
        self.fetch_paper_pdf_url_by_doi(url).await
    }
    /// Fetches the pdf url of the paper with the given doi using the given sci-hub base url.
    pub async fn fetch_paper_pdf_url_by_base_url_and_doi(
        &self,
        base_url: &Url,
        doi: &str,
    ) -> Result<Url, Error> {
        let url = Self::scihub_url_from_base_url_and_doi(base_url, doi)?;
        self.fetch_paper_pdf_url_from_scihub_url(url).await
    }
    /// Fetches the pdf url of the paper from the given scihub url.
    pub async fn fetch_paper_pdf_url_from_scihub_url(&self, url: Url) -> Result<Url, Error> {
        let client = Client::builder()
            .redirect(redirect::Policy::none())
            .build()?;

        let response = client
            .get(url.clone())
            .header(
                header::USER_AGENT,
                "Mozilla/5.0 (Android 4.4; Mobile; rv:42.0) Gecko/42.0 Firefox/42.0",
            ) // "disguise" as mobile
            .send()
            .await?;

        response
            .headers()
            .get(header::LOCATION)
            .ok_or_else(|| Error::SciHubParse(
                "Received unexpected response from sci-hub.",
            ))?
            .to_str()
            .or_else(|_| Err(Error::SciHubParse(
                "Received malformed pdf url from sci-hub.",
            )))
            .map(|pdf_url| Self::convert_protocol_relative_url_to_absolute(pdf_url, &url))
            .and_then(|url_str| Url::parse(&url_str).map_err(|e| e.into()))
            .and_then(|url| {
                if url.domain().map_or(false, |e| e.contains("sci-hub")) {
                    return Ok(url);
                } else {
                    return Err(Error::Other("Redirected to invalid site."));
                }
            })
    }

    async fn fetch_html_document(&self, url: Url) -> Result<Html, Error> {
        let text = self
            .client
            .get(url)
            .header(header::ACCEPT, "text/html")
            .send()
            .await?
            .text()
            .await?;
        Ok(Html::parse_document(&text))
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

pub struct WeightedUrl {
    pub url: Url,
    weight: u32,
}
impl PartialEq for WeightedUrl {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for WeightedUrl {}
impl PartialOrd for WeightedUrl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.weight.partial_cmp(&other.weight)
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
impl Into<Url> for WeightedUrl {
    fn into(self) -> Url {
        self.url
    }
}
