use reqwest::Client;
use scihub_scraper::{Scraper, fetch_base_urls};
use tokio::runtime::Runtime;
use url::Url;

const TEST_DOI: &str = "10.1016/j.tplants.2018.11.001";
const TEST_TITLE: &str = "Capsaicinoids: Pungency beyond Capsicum";

#[test]
fn finds_scihub_base_urls() {
    let runtime = Runtime::new().unwrap();
    let client = Client::new();
    let base_urls = runtime.block_on(fetch_base_urls(&client)).unwrap();
    assert!(!base_urls.is_empty());
}

#[test]
fn creates_valid_scihub_urls() {
    let base_url = Url::parse("http://sci-hub.test").unwrap();
    Scraper::scihub_url_from_base_url_and_doi(&base_url, TEST_DOI).unwrap();
}

#[test]
fn fetches_paper() {
    let mut runtime = Runtime::new().unwrap();
    let mut scihub = runtime
        .block_on(Scraper::with_auto_detected_base_urls())
        .unwrap();
    let paper = runtime
        .block_on(scihub.fetch_paper_by_doi(TEST_DOI))
        .unwrap();
    assert_eq!(paper.doi, TEST_DOI);
    assert_eq!(paper.title, TEST_TITLE);
    assert!(!paper.other_versions.is_empty());
    check_pdf_url(paper.download_url, &mut runtime);
}

#[test]
fn fetches_pdf_url_direct() {
    let mut runtime = Runtime::new().unwrap();
    let mut scihub = runtime
        .block_on(Scraper::with_auto_detected_base_urls())
        .unwrap();
    let pdf_url = runtime
        .block_on(scihub.fetch_paper_pdf_url_by_doi(TEST_DOI))
        .unwrap();
    check_pdf_url(pdf_url, &mut runtime);
}

fn check_pdf_url(pdf_url: Url, runtime: &mut Runtime) {
    assert!(
        pdf_url.path().ends_with(".pdf"),
        "Pdf url path does not end with '.pdf'"
    );
    let client = Client::new();
    assert_eq!(
        runtime
            .block_on(client.get(pdf_url).header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:146.0) Gecko/20100101 Firefox/146.0",
            )
            .send())
            .unwrap()
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "application/pdf"
    );
}
