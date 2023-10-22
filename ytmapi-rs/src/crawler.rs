use serde::de::DeserializeOwned;

use crate::{error::ParseTarget, Error, Result};
use std::{slice::IterMut, sync::Arc};

#[derive(Clone, PartialEq, Debug)]
pub enum JsonPath {
    Pointer(String),
    IndexNum(usize),
}
#[derive(Clone, Default, PartialEq, Debug)]
struct PathList {
    list: Vec<JsonPath>,
}
#[derive(Clone, PartialEq, Debug)]
pub struct JsonCrawler {
    // Source is wrapped in an Arc as we are going to pass ownership when returning an error and we want it to be thread safe.
    source: Arc<serde_json::Value>,
    crawler: serde_json::Value,
    path: PathList,
}
pub struct JsonCrawlerBorrowed<'a> {
    // Source is wrapped in an Arc as we are going to pass ownership when returning an error and we want it to be thread safe.
    source: Arc<serde_json::Value>,
    crawler: &'a mut serde_json::Value,
    path: PathList,
}
pub struct JsonCrawlerArrayIterMut<'a> {
    source: Arc<serde_json::Value>,
    array: IterMut<'a, serde_json::Value>,
    path: PathList,
    cur: usize,
    len: usize,
}
impl From<&JsonPath> for String {
    fn from(value: &JsonPath) -> Self {
        match value {
            JsonPath::Pointer(p) => p.to_owned(),
            JsonPath::IndexNum(i) => format! {"/{i}"},
        }
    }
}
impl JsonPath {
    pub fn pointer<S: Into<String>>(path: S) -> Self {
        JsonPath::Pointer(path.into())
    }
}
impl PathList {
    fn push(&mut self, path: JsonPath) {
        self.list.push(path)
    }
    fn pop(&mut self) -> Option<JsonPath> {
        self.list.pop()
    }
}
impl From<&PathList> for String {
    fn from(value: &PathList) -> Self {
        let mut path = String::new();
        for p in &value.list {
            path.push_str(String::from(p).as_str());
        }
        path
    }
}

impl<'a> JsonCrawlerArrayIterMut<'a> {
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a> Iterator for JsonCrawlerArrayIterMut<'a> {
    type Item = JsonCrawlerBorrowed<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.cur += 1;
        if self.len <= self.cur {
            return None;
        }
        self.path.pop();
        self.path.push(JsonPath::IndexNum(self.cur));
        Some(JsonCrawlerBorrowed {
            // Ideally there should be a Borrowed version of this struct - otherwise we need to clone every time here.
            source: self.source.clone(),
            crawler: self.array.next()?,
            // As above - needs to be cloned every time.
            path: self.path.clone(),
        })
    }
}

impl<'a> JsonCrawlerBorrowed<'a> {
    pub fn into_array_iter_mut(self) -> Result<JsonCrawlerArrayIterMut<'a>> {
        let json_array = self
            .crawler
            .as_array_mut()
            .ok_or_else(|| Error::parsing(&self.path, self.source.clone(), ParseTarget::Array))?;
        let len = json_array.len();
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(0));
        Ok(JsonCrawlerArrayIterMut {
            source: self.source,
            len,
            array: json_array.iter_mut(),
            path: path_clone,
            cur: 0,
        })
    }
    pub fn as_array_iter_mut(&mut self) -> Result<JsonCrawlerArrayIterMut<'_>> {
        let json_array = self
            .crawler
            .as_array_mut()
            .ok_or_else(|| Error::parsing(&self.path, self.source.clone(), ParseTarget::Array))?;
        let len = json_array.len();
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(0));
        Ok(JsonCrawlerArrayIterMut {
            source: self.source.clone(),
            len,
            array: json_array.iter_mut(),
            path: path_clone,
            cur: 0,
        })
    }
    pub fn borrow_pointer<S: AsRef<str>>(&mut self, path: S) -> Result<JsonCrawlerBorrowed<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    pub fn navigate_pointer<S: AsRef<str>>(self, path: S) -> Result<JsonCrawlerBorrowed<'a>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        let crawler = self
            .crawler
            .pointer_mut(path.as_ref())
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(Self {
            source: self.source,
            crawler,
            path: path_clone,
        })
    }
    // XXX: Temporary function until code refactored.
    #[deprecated = "Temporary function - in future pass navigator around instead"]
    pub fn _take_json_pointer(&mut self, path: &str) -> Result<serde_json::Value> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path));
        self.crawler
            .pointer_mut(path)
            .map(|v| v.take())
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))
    }
    pub fn take_value<T: DeserializeOwned>(&mut self) -> Result<T> {
        serde_json::from_value(self.crawler.take())
            // XXX: ParseTarget String is incorrect
            .map_err(|_| Error::parsing(&self.path, self.source.clone(), ParseTarget::String))
    }
    pub fn take_value_pointer<T: DeserializeOwned, S: AsRef<str>>(&mut self, path: S) -> Result<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path.as_ref()));
        serde_json::from_value(
            self.crawler
                .pointer_mut(path.as_ref())
                .map(|v| v.take())
                .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?,
        )
        // XXX: ParseTarget String is incorrect
        .map_err(|_| Error::parsing(&path_clone, self.source.clone(), ParseTarget::String))
    }
    pub fn path_exists(&self, path: &str) -> bool {
        self.crawler.pointer(path).is_some()
    }
    pub fn get_source(&self) -> &serde_json::Value {
        &*self.source
    }
}

impl JsonCrawler {
    // TODO: Implement into_array_iter_mut.
    pub fn as_array_iter_mut(&mut self) -> Result<JsonCrawlerArrayIterMut<'_>> {
        let json_array = self
            .crawler
            .as_array_mut()
            .ok_or_else(|| Error::parsing(&self.path, self.source.clone(), ParseTarget::Array))?;
        let len = json_array.len();
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::IndexNum(0));
        Ok(JsonCrawlerArrayIterMut {
            source: self.source.clone(),
            len,
            array: json_array.iter_mut(),
            path: path_clone,
            cur: 0,
        })
    }
    pub fn borrow_pointer(&mut self, path: &str) -> Result<JsonCrawlerBorrowed<'_>> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::Pointer(path.to_owned()));
        let crawler = self
            .crawler
            .pointer_mut(path)
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?;
        Ok(JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler,
            path: path_clone,
        })
    }
    pub fn borrow_mut(&mut self) -> JsonCrawlerBorrowed<'_> {
        JsonCrawlerBorrowed {
            source: self.source.clone(),
            crawler: &mut self.crawler,
            path: self.path.to_owned(),
        }
    }
    pub fn path_exists(&self, path: &str) -> bool {
        self.crawler.pointer(path).is_some()
    }
    pub fn navigate_pointer(self, new_path: &str) -> Result<Self> {
        let Self {
            source,
            crawler: mut old_crawler,
            mut path,
        } = self;
        path.push(JsonPath::pointer(new_path));
        let crawler = old_crawler
            .pointer_mut(new_path)
            .map(|v| v.take())
            .ok_or_else(|| Error::navigation(&path, source.clone()))?;
        Ok(Self {
            source,
            crawler,
            path,
        })
    }
    pub fn from_json(json: serde_json::Value) -> Self {
        let crawler = json.clone();
        Self {
            source: Arc::new(json),
            crawler,
            path: PathList::default(),
        }
    }
    pub fn take_value<T: DeserializeOwned>(&mut self) -> Result<T> {
        serde_json::from_value(self.crawler.take())
            // XXX: ParseTarget String is incorrect
            .map_err(|_| Error::parsing(&self.path, self.source.clone(), ParseTarget::String))
    }
    pub fn take_value_pointer<T: DeserializeOwned>(&mut self, path: &str) -> Result<T> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path));
        serde_json::from_value(
            self.crawler
                .pointer_mut(path)
                .map(|v| v.take())
                .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))?,
        )
        // XXX: ParseTarget String is incorrect
        .map_err(|_| Error::parsing(&path_clone, self.source.clone(), ParseTarget::String))
    }
    #[deprecated = "Temporary function until code refactored"]
    pub fn _take_json_pointer(&mut self, path: &str) -> Result<serde_json::Value> {
        let mut path_clone = self.path.clone();
        path_clone.push(JsonPath::pointer(path));
        self.crawler
            .pointer_mut(path)
            .map(|v| v.take())
            .ok_or_else(|| Error::navigation(&path_clone, self.source.clone()))
    }
    pub fn get_source(&self) -> &serde_json::Value {
        &*self.source
    }
}
