use scihub_scraper::*;
use tokio::runtime::Runtime;
use url::Url;

#[test]
fn finds_scihub_base_urls() -> Result<(), Box<dyn std::error::Error>> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new()?;
    runtime.block_on(scihub.fetch_base_urls())?;
    assert!(scihub.base_urls.is_some());
    assert!(!scihub.base_urls.as_ref().unwrap().is_empty());
    Ok(())
}

#[test]
fn creates_valid_scihub_urls() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = Url::parse("http://sci-hub.test")?;
    SciHubScraper::url_from_base_url_and_doi(&base_url, "10.1016/j.tplants.2018.11.001")?;
    Ok(())
}

#[test]
fn finds_scihub_pdf_url() -> Result<(), Box<dyn std::error::Error>> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new()?;
    let pdf_url_str = runtime.block_on(scihub.fetch_pdf_url_from_doi("10.1016/j.tplants.2018.11.001"))?;
    let pdf_url = Url::parse(&pdf_url_str)?;
    assert!(pdf_url.path().ends_with(".pdf"), "Pdf url path does not end with '.pdf'");
    println!("{:?}", pdf_url);
    assert_eq!(
        runtime.block_on(reqwest::get(pdf_url))?
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .unwrap()
        .to_str()?, "application/pdf");
    Ok(())
}

#[test]
fn fetches_scihub_pdf_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new()?;
    let pdf_bytes = runtime.block_on(scihub.fetch_pdf_bytes_from_doi("10.1016/j.tplants.2018.11.001"))?;
    assert!(!pdf_bytes.is_empty());
    Ok(())
}
