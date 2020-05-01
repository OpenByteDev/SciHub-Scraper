use quick_error::{quick_error};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Reqwest(err: reqwest::Error) {
            from()
            display("reqwest error: {}", err)
            cause(err)
        }
        UrlParse(err: url::ParseError) {
            from()
            display("url parse error: {}", err)
            cause(err)
        }
        Other(descr: &'static str) {
            display("error {}", descr)
        }
        SciHubParse(descr: &'static str) {
            display("error {}", descr)
        }
    }
}
