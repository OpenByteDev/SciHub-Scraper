use scihub_scraper::*;
use tokio::runtime::Runtime;
use url::Url;

#[test]
fn finds_scihub_base_urls() -> Result<(), Error> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new().unwrap();
    runtime.block_on(scihub.fetch_scihub_base_urls())?;
    assert!(scihub.scihub_base_urls.is_some());
    assert!(!scihub.scihub_base_urls.as_ref().unwrap().is_empty());
    Ok(())
}

#[test]
fn creates_valid_scihub_urls() -> Result<(), url::ParseError> {
    let base_url = Url::parse("http://sci-hub.test").unwrap();
    SciHubScraper::url_from_base_url_and_doi(&base_url, "10.1016/j.tplants.2018.11.001")?;
    Ok(())
}

#[test]
fn finds_scihub_pdf_url() -> Result<(), Error> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new().unwrap();
    let pdf_url_str = runtime.block_on(scihub.fetch_pdf_url_from_doi("10.1016/j.tplants.2018.11.001"))?;
    let pdf_url = Url::parse(&pdf_url_str).unwrap();
    assert!(pdf_url.path().ends_with(".pdf"), "Pdf url path does not end with '.pdf'");
    Ok(())
}
