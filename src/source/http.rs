use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use symphonia::core::io::MediaSource;

/// A seekable HTTP body that symphonia can probe and decode directly, so an
/// online track starts playing on the first packet instead of after the whole
/// file has been downloaded.
///
/// Every seek drops the open body and reopens with a `Range` header. Probing
/// does a handful of those; playback then reads straight through.
pub struct HttpStream {
    agent: ureq::Agent,
    url: String,
    position: u64,
    length: Option<u64>,
    body: Option<Box<dyn Read + Send + Sync>>,
}

impl HttpStream {
    pub fn open(url: &str) -> Option<HttpStream> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(30))
            .build();
        let mut stream = HttpStream {
            agent,
            url: url.to_string(),
            position: 0,
            length: None,
            body: None,
        };
        stream.reopen()?;
        stream.length?;
        Some(stream)
    }

    fn reopen(&mut self) -> Option<()> {
        let response = self
            .agent
            .get(&self.url)
            .set("Range", &format!("bytes={}-", self.position))
            .call()
            .ok()?;

        if self.length.is_none() {
            self.length = total_length(&response, self.position);
        }
        self.body = Some(response.into_reader());
        Some(())
    }
}

fn total_length(response: &ureq::Response, offset: u64) -> Option<u64> {
    if let Some(range) = response.header("Content-Range") {
        if let Some((_, total)) = range.rsplit_once('/') {
            if let Ok(n) = total.trim().parse::<u64>() {
                return Some(n);
            }
        }
    }
    response
        .header("Content-Length")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(|n| n + offset)
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.body.is_none() && self.reopen().is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "stream closed",
            ));
        }
        let body = self.body.as_mut().expect("body opened above");
        let read = body.read(buf)?;
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for HttpStream {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let target = match to {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.position as i64 + n,
            SeekFrom::End(n) => self.length.unwrap_or(0) as i64 + n,
        };
        if target < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before start",
            ));
        }
        let target = target as u64;
        if target != self.position {
            self.position = target;
            self.body = None;
        }
        Ok(self.position)
    }
}

impl MediaSource for HttpStream {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.length
    }
}
