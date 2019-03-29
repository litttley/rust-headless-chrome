use futures::{Poll, Async};
use websocket::futures::{Future, Stream, Fuse};
use tokio::timer::{Interval};
use super::one_page::{OnePage};
use super::page_message::{PageMessage};
use failure::{self, Error, Fail};
use std::time::{Duration, Instant};
use log::*;

/// An adapter for merging the output of two streams.
///
/// The merged stream produces items from either of the underlying streams as
/// they become available, and the streams are polled in a round-robin fashion.
/// Errors, however, are not merged: you get at most one error at a time.
// #[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct IntervalOnePage {
    interval_page_message: IntervalPageMessage,
    pub one_page: OnePage,
    end_of_sleep: Option<Instant>,
    flag: bool,
}

impl IntervalOnePage {
    pub fn new(duration: Duration, one_page: OnePage) -> Self {
        let interval_page_message = IntervalPageMessage {
            interval: Interval::new_interval(duration),
        };
        Self {
            interval_page_message,
            one_page,
            end_of_sleep: None,
            flag: false,
        }
    }
}


#[derive(Debug)]
pub struct IntervalPageMessage {
    interval: Interval,
}

impl Stream for IntervalPageMessage {
    type Item = PageMessage;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            if let Some(_) = try_ready!(self.interval.poll()) {
                return Ok(Async::Ready(Some(PageMessage::Interval)));
            }
        }
    }
}

impl IntervalOnePage {
    pub fn sleep(&mut self, duration:Duration) {
        if self.end_of_sleep.is_none() {
            self.end_of_sleep = Some(Instant::now() + duration);
        }
    }

    pub fn send_page_message(&mut self, item: PageMessage) -> Poll<Option<PageMessage>, failure::Error> {
        info!("{:?}", item);
        match &item {
            PageMessage::Interval => {
                if let Some(inst) = self.end_of_sleep {
                    if Instant::now() > inst {
                        info!("sleep over>>>>>>>>>>>>>>>");
                        self.end_of_sleep = None;
                    }
                }
            },
            _ => ()
        }
        return Ok(Some(item).into());
    }
}


impl Stream for IntervalOnePage {
    type Item = PageMessage;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let (a, b) = if self.flag {
            (&mut self.one_page as &mut Stream<Item=_, Error=_>,
             &mut self.interval_page_message as &mut Stream<Item=_, Error=_>)
        } else {
            (&mut self.interval_page_message as &mut Stream<Item=_, Error=_>,
             &mut self.one_page as &mut Stream<Item=_, Error=_>)
        };
        self.flag = !self.flag;
        let a_done = match a.poll()? {
            Async::Ready(Some(item)) => return self.send_page_message(item),
            Async::Ready(None) => true,
            Async::NotReady => false,
        };

        match b.poll()? {
            Async::Ready(Some(item)) => {
                // If the other stream isn't finished yet, give them a chance to
                // go first next time as we pulled something off `b`.
                if !a_done {
                    self.flag = !self.flag;
                }
                self.send_page_message(item)
            }
            Async::Ready(None) if a_done => Ok(None.into()),
            Async::Ready(None) | Async::NotReady => Ok(Async::NotReady),
        }
    }
}