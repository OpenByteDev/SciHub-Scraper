# SciHub Scraper

[![crates.io](https://img.shields.io/crates/v/scihub-scraper.svg)](https://crates.io/crates/scihub-scraper)
[![Documentation](https://docs.rs/scihub-scraper/badge.svg)](https://docs.rs/scihub-scraper)
[![dependency status](https://deps.rs/repo/github/openbytedev/scihub-scraper/status.svg)](https://deps.rs/repo/github/openbytedev/scihub-scraper)
[![MIT](https://img.shields.io/crates/l/scihub-scraper.svg)](https://github.com/OpenByteDev/SciHub-Scraper/blob/master/LICENSE)

SciHub Scraper can be used to scrap paper info including its pdf url from sci-hub.
 
Sci-hub domains are automatically fetched from [sci-hub.now.sh](https://sci-hub.now.sh/), 
but can also be manually provided by using `SciHubScraper::with_base_urls`

## Usage
To extract all available information about a paper use `SciHubScraper.fetch_paper_by_doi` and associated methods:
```rust
let mut scraper = SciHubScraper::new();
let paper = scraper.fetch_paper_by_doi("10.1016/j.tplants.2018.11.001").await?;
println!("Title = {}", paper.title);
println!("PDF Url = {}", paper.download_url);
```

Alernatively `SciHubScraper.fetch_paper_pdf_url_by_doi` and associated methods can also be used to extract the PDF Url of a paper.
It only extracts the url using a different method and is therefore faster.

```rust
let mut scraper = SciHubScraper::new();
let pdf_url = scraper.fetch_paper_pdf_url_by_doi("10.1016/j.tplants.2018.11.001").await?;
println!("PDF Url = {}", pdf_url);
```

## License
Licensed under MIT license ([LICENSE](https://github.com/OpenByteDev/SciHub-Scraper/blob/master/LICENSE) or http://opensource.org/licenses/MIT)
