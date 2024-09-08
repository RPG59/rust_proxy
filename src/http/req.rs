use url::{ParseError, Url};

use std::collections::HashMap;

pub struct RpgxRequest {
    pub url: Url,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl RpgxRequest {
    pub fn to_vec(&self) -> String {
        let http_header = format!("{} {} HTTP/1.1", self.method, self.url.path());
        let headers = self
            .headers
            .iter()
            .map(|x| format!("{}:{}\r\n", x.0, x.1))
            .collect();

        vec![http_header, headers, "\r\n".to_string()].join("\r\n")
    }
}
