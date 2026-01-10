# SciHub Scraper

[![crates.io](https://img.shields.io/crates/v/scihub-scraper.svg)](https://crates.io/crates/scihub-scraper)
[![Documentation](https://docs.rs/scihub-scraper/badge.svg)](https://docs.rs/scihub-scraper)
[![dependency status](https://deps.rs/repo/github/openbytedev/scihub-scraper/status.svg)](https://deps.rs/repo/github/openbytedev/scihub-scraper)
[![MIT](https://img.shields.io/crates/l/scihub-scraper.svg)](https://github.com/OpenByteDev/SciHub-Scraper/blob/master/LICENSE)

SciHub Scraper can be used to scrap paper info including its pdf url from sci-hub.

## Usage
To fetch information about a paper use `Scraper::fetch_paper_by_doi`:
```rust
let mut scraper = Scraper::with_auto_detected_base_urls().await.unwrap();
let paper = scraper.fetch_paper_by_doi("10.1016/j.tplants.2018.11.001").await.unwrap();
println!("Title = {}", paper.title);
println!("PDF Url = {}", paper.download_url);
```

Sci-hub domains can be automatically fetched with `Scraper::with_auto_detected_base_urls`,
or manually provided by using `Scraper::with_base_urls`.


## License
Licensed under MIT license ([LICENSE](https://github.com/OpenByteDev/SciHub-Scraper/blob/master/LICENSE) or http://opensource.org/licenses/MIT)
