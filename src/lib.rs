use reqwest::{Client};
use scraper::{Html, Selector};
use quick_error::{quick_error};
use url::Url;

pub struct SciHubScraper {
    client: Client,
    pub scihub_base_urls: Option<Vec<Url>>
}

impl SciHubScraper {
    pub fn new() -> Self {
        SciHubScraper {
            client: Client::new(),
            scihub_base_urls: None
        }
    }
    pub async fn fetch_scihub_base_urls(&mut self) -> Result<(), Error> {
        let scihub_now_url = Url::parse("https://sci-hub.now.sh/").unwrap();
        let document = self.fetch_html_document(scihub_now_url).await?;

        let link_selector = Selector::parse("a[href]").unwrap();
        let mut domains: Vec<Url> = document.select(&link_selector)
            .filter_map(|node| node.value().attr("href"))
            .filter_map(|href| Url::parse(href).ok())
            .filter(|url| url.domain().map_or(false, |e| e.starts_with("sci-hub") && !e.ends_with("now.sh")))
            .collect();
        domains.dedup();

        self.scihub_base_urls = Some(domains);
        Ok(())
    }
    async fn ensure_scihub_base_urls(&mut self) -> Result<(), Error> {
        if self.scihub_base_urls.is_none() {
            self.fetch_scihub_base_urls().await?;
        }
        if let Some(vec) = &self.scihub_base_urls {
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
        self.scihub_base_urls.as_ref()
            .map(|base_urls|
                base_urls.iter()
                    .filter_map(|base_url| Self::url_from_base_url_and_doi(base_url, doi).ok())
                    .collect())
    }
    pub async fn fetch_pdf_url_from_doi(&mut self, doi: &str) -> Result<String, Error> {
        self.ensure_scihub_base_urls().await?;

        let pdf_frame_selector = Selector::parse("iframe#pdf[src]").unwrap();
        for url in self.urls_from_doi(doi).unwrap() {
            match self.fetch_html_document(url.clone()).await {
                Ok(document) => {
                    if let Some(pdf_url) = document.select(&pdf_frame_selector)
                        .next()
                        .map(|node| node.value().attr("src"))
                        .flatten() {
                            if pdf_url.starts_with("//") {
                                return Ok(format!("{}:{}", url.scheme(), pdf_url));
                            }
                            return Ok(String::from(pdf_url));
                    }
                },
                Err(e) => {
                    println!("{:?}", e);
                    // TODO remove or mark domain as broken.
                }
            }
        }
        Err(Error::Other("Invalid doi or no working sci-hub mirror found"))
    }
    async fn fetch_html_document(&self, url: Url) -> Result<Html, Error> {
        let text = self.client
            .get(url)
            .header("Accepts", "text/html")
            .send().await?
            .text().await?;
        Ok(Html::parse_document(&text))
    }

}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: reqwest::Error) {
            from()
            display("Reqwest error: {}", err)
            cause(err)
        }
        Other(descr: &'static str) {
            display("Error {}", descr)
        }
    }
}

