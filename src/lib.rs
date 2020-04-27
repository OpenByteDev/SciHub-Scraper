use reqwest::{Client};
use scraper::{Html, Selector};
use quick_error::{quick_error};
use url::Url;
use bytes::Bytes;

pub struct SciHubScraper {
    client: Client,
    pub base_urls: Option<Vec<Url>>
}

impl SciHubScraper {
    pub fn new() -> Self {
        SciHubScraper {
            client: Client::new(),
            base_urls: None
        }
    }
    pub fn with_base_urls(base_urls: Vec<Url>) -> Self {
        SciHubScraper {
            client: Client::new(),
            base_urls: Some(base_urls)
        }
    }
    pub async fn fetch_base_urls(&mut self) -> Result<(), Error> {
        let scihub_now_url = Url::parse("https://sci-hub.now.sh/").unwrap();
        let document = self.fetch_html_document(scihub_now_url).await?;

        let link_selector = Selector::parse("a[href]").unwrap();
        let mut domains: Vec<Url> = document.select(&link_selector)
            .filter_map(|node| node.value().attr("href"))
            .filter_map(|href| Url::parse(href).ok())
            .filter(|url| url.domain().map_or(false, |e| e.starts_with("sci-hub") && !e.ends_with("now.sh")))
            .collect();
        domains.dedup();

        self.base_urls = Some(domains);
        Ok(())
    }
    async fn ensure_base_urls(&mut self) -> Result<(), Error> {
        if self.base_urls.is_none() {
            self.fetch_base_urls().await?;
        }
        if let Some(vec) = &self.base_urls {
            if vec.is_empty() {
                return Err(Error::Other("No sci-hub domains found."));
            }
            Ok(())
        } else {
            Err(Error::Other("Failed to load sci-hub domains."))
        }
    }
    pub fn url_from_base_url_and_doi(base_url: &Url, doi: &str) -> Result<Url, url::ParseError> {
        base_url.join(doi)
    }
    pub fn urls_from_doi<'a>(&self, doi: &'a str) -> Option<Vec<Url>> {
        self.base_urls.as_ref()
            .map(|base_urls|
                base_urls.iter()
                    .filter_map(|base_url| Self::url_from_base_url_and_doi(base_url, doi).ok())
                    .collect())
    }
    pub async fn fetch_pdf_url_from_doi(&mut self, doi: &str) -> Result<String, Error> {
        self.ensure_base_urls().await?;

        for base_url in self.base_urls.as_ref().unwrap() {
            let pdf_url = self.fetch_pdf_url_from_base_url_and_doi(&base_url, &doi).await?;
            return Ok(pdf_url);
        }
        Err(Error::Other("Invalid doi or no working sci-hub mirror found"))
    }
    pub async fn fetch_pdf_url_from_base_url_and_doi(&self, base_url: &Url, doi: &str) -> Result<String, Error> {
        let url = Self::url_from_base_url_and_doi(&base_url, doi)?;
        self.fetch_pdf_url_from_scihub_url(&url).await
    }
    pub async fn fetch_pdf_url_from_scihub_url(&self, url: &Url) -> Result<String, Error> {
        let pdf_frame_selector = Selector::parse("iframe#pdf[src]").unwrap();
        let document = self.fetch_html_document(url.clone()).await?;
        let pdf_url = document.select(&pdf_frame_selector)
            .filter_map(|node| node.value().attr("src"))
            .next()
            .ok_or(Error::Other("Pdf url not found in page."))?;
        if pdf_url.starts_with("//") {
            return Ok(format!("{}:{}", url.scheme(), pdf_url));
        }
        return Ok(String::from(pdf_url));
    }
    pub async fn fetch_pdf_bytes_from_doi(&mut self, doi: &str) -> Result<Bytes, Error> {
        let pdf_url_str = self.fetch_pdf_url_from_doi(doi).await?;
        let pdf_url = Url::parse(&pdf_url_str)?;
        self.fetch_pdf_document(pdf_url).await
    }
    pub async fn fetch_pdf_bytes_from_base_url_and_doi(&self, base_url: &Url, doi: &str) -> Result<Bytes, Error> {
        let pdf_url_str = self.fetch_pdf_url_from_base_url_and_doi(base_url, doi).await?;
        let pdf_url = Url::parse(&pdf_url_str)?;
        self.fetch_pdf_document(pdf_url).await
    }
    pub async fn fetch_pdf_bytes_from_scihub_url(&self, url: &Url) -> Result<Bytes, Error> {
        let pdf_url_str = self.fetch_pdf_url_from_scihub_url(url).await?;
        let pdf_url = Url::parse(&pdf_url_str)?;
        self.fetch_pdf_document(pdf_url).await
    }
    async fn fetch_html_document(&self, url: Url) -> Result<Html, Error> {
        let text = self.client
            .get(url)
            .header("Accepts", "text/html")
            .send().await?
            .text().await?;
        Ok(Html::parse_document(&text))
    }
    async fn fetch_pdf_document(&self, url: Url) -> Result<Bytes, Error> {
        let bytes = self.client.get(url)
            .send().await?
            .bytes().await?;
        Ok(bytes)
    }

}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Reqwest(err: reqwest::Error) {
            from()
            display("Reqwest error: {}", err)
            cause(err)
        }
        UrlParse(err: url::ParseError) {
            from()
            display("Url parse error: {}", err)
            cause(err)
        }
        Other(descr: &'static str) {
            display("Error {}", descr)
        }
    }
}

