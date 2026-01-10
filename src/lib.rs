//! # SciHub Scraper
//!
//! SciHub Scraper can be used to scrap paper info including its pdf url from sci-hub.
//!
//! ## Usage
//! To fetch information about a paper use [`Scraper::fetch_paper_by_doi`]:
//! ```rust
//! # use scihub_scraper::Scraper;
//! # async fn run() {
//! let mut scraper = Scraper::with_auto_detected_base_urls().await.unwrap();
//! let paper = scraper.fetch_paper_by_doi("10.1016/j.tplants.2018.11.001").await.unwrap();
//! println!("Title = {}", paper.title);
//! println!("PDF Url = {}", paper.download_url);
//! # }
//! # use tokio::runtime::Runtime;
//! # fn main() {
//! # let mut runtime = Runtime::new().unwrap();
//! # runtime.block_on(run());
//! # }
//! ```
//!
//! Sci-hub domains can be automatically fetched with [`Scraper::with_auto_detected_base_urls`],
//! or manually provided by using [`Scraper::with_base_urls`].

pub mod error;
pub mod scraper;
mod url_pool;

pub use crate::{error::*, scraper::*};
