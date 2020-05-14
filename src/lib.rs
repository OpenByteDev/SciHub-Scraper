//! # SciHub Scraper
//!
//! SciHub Scraper can be used to scrap paper info including its pdf url from sci-hub.
//!  
//! Sci-hub domains are automatically fetched from [sci-hub.now.sh](https://sci-hub.now.sh/),
//! but can also be manually provided by using `SciHubScraper::with_base_urls`
//!
//! ## Usage
//! To extract all available information about a paper use `SciHubScraper.fetch_paper_by_doi` and associated methods:
//! ```rust
//! # use scihub_scraper::{SciHubScraper, Error};
//! #
//! # async fn run() -> Result<(), Error> {
//! let mut scraper = SciHubScraper::new();
//! let paper = scraper.fetch_paper_by_doi("10.1016/j.tplants.2018.11.001").await?;
//! println!("Title = {}", paper.title);
//! println!("PDF Url = {}", paper.download_url);
//! # Ok(())
//! # }
//! ```
//!
//! Alernatively `SciHubScraper.fetch_paper_pdf_url_by_doi` and associated methods can also be used to extract only the pdf url of a paper.
//! It extracts the url using a different method and is therefore faster but provides no additional information.
//!
//! ```rust
//! # use scihub_scraper::{SciHubScraper, Error};
//! #
//! # async fn run() -> Result<(), Error> {
//! let mut scraper = SciHubScraper::new();
//! let pdf_url = scraper.fetch_paper_pdf_url_by_doi("10.1016/j.tplants.2018.11.001").await?;
//! println!("PDF Url = {}", pdf_url);
//! # Ok(())
//! # }
//! ```

#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod scraper;

pub use crate::error::*;
pub use crate::scraper::*;
