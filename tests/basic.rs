use scihub_scraper::SciHubScraper;
use tokio::runtime::Runtime;
use url::Url;

const TEST_DOI: &str = "10.1016/j.tplants.2018.11.001";
const TEST_TITLE: &str = "Capsaicinoids: Pungency beyond Capsicum. Trends in Plant Science";

#[test]
fn finds_scihub_base_urls() -> Result<(), Box<dyn std::error::Error>> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new()?;
    runtime.block_on(scihub.fetch_base_urls())?;
    assert!(!scihub.base_urls.is_empty());
    Ok(())
}

#[test]
fn creates_valid_scihub_urls() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = Url::parse("http://sci-hub.test")?;
    SciHubScraper::scihub_url_from_base_url_and_doi(&base_url, TEST_DOI)?;
    Ok(())
}

#[test]
fn fetches_paper() -> Result<(), Box<dyn std::error::Error>> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new()?;
    let paper = runtime.block_on(scihub.fetch_paper_by_doi(TEST_DOI))?;
    assert_eq!(paper.doi, TEST_DOI);
    assert_eq!(paper.title, TEST_TITLE);
    assert!(!paper.other_versions.is_empty());
    check_pdf_url(&paper.download_url, &mut runtime)
}

#[test]
fn fetches_pdf_url_direct() -> Result<(), Box<dyn std::error::Error>> {
    let mut scihub = SciHubScraper::new();
    let mut runtime = Runtime::new()?;
    let pdf_url = runtime.block_on(scihub.fetch_paper_pdf_url_by_doi(TEST_DOI))?;
    check_pdf_url(&pdf_url, &mut runtime)
}

fn check_pdf_url(pdf_url: &str, runtime: &mut Runtime) -> Result<(), Box<dyn std::error::Error>> {
    let pdf_url = Url::parse(pdf_url)?;
    assert!(
        pdf_url.path().ends_with(".pdf"),
        "Pdf url path does not end with '.pdf'"
    );
    assert_eq!(
        runtime
            .block_on(reqwest::get(pdf_url))?
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()?,
        "application/pdf"
    );
    Ok(())
}
