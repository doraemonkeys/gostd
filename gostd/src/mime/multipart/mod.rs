//! Package multipart implements MIME multipart parsing, as defined in RFC 2046.
//! The implementation is sufficient for HTTP (RFC 2388) and the multipart bodies generated by popular browsers.
//!
//! <details class="rustdoc-toggle top-doc">
//! <summary class="docblock">zh-cn</summary>
//! multipart实现了MIME的multipart解析，参见RFC 2046。该实现适用于HTTP（RFC 2388）和常见浏览器生成的multipart主体。
//! </details>
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

// #[cfg(test)]
// mod tests;
use crate::io::*;
use crate::{builtin::*, bytes, io, strings};
use rand::RngCore;
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::io::Error;

/// A Writer generates multipart messages.
/// <details class="rustdoc-toggle top-doc">
/// <summary class="docblock">zh-cn</summary>
/// Writer类型用于生成multipart信息。
/// </details>
///
/// # Example
///
/// ```
/// use gostd::bytes;
/// use gostd::mime::multipart::Writer;
///     let mut body = bytes::Buffer::new();
///     let mut w = Writer::new(&mut body);
///     w.WriteField("requestId", "12121231231")?;
///     w.WriteField("testTime", "2022-01-22 18:00:00")?;
///     w.WriteField("checkTime", "2022-01-22 22:00:00")?;
///     w.WriteField("auditTime", "2022-01-22 23:00:00")?;
///     w.WriteField("tubeCode", "QCGD99SDF")?;
///     w.WriteField("testRatio", "1")?;
///     w.WriteField("name", "刘某某")?;
///     w.WriteField("sex", "1")?;
///     w.WriteField("birthdate", "2000-02-02")?;
///     w.WriteField("address", "北京市丰台区")?;
///     w.WriteField("phoneNumber", "188xx8439xx")?;
///     w.WriteField("cardType", "护照")?;
///     w.WriteField("cardNumber", "xxxx")?;
///     w.WriteField("testResult", "0")?;
///     w.WriteField("testUserName", "xxx")?;
///     w.WriteField("checkUserName", "xxx")?;
///     w.Close()?;
///     println!("{}", w.FormDataContentType());
///     println!("{}", body.String());
///     Ok(())
/// ```
#[derive(Debug)]
pub struct Writer<'a, W>
where
    W: io::Writer,
{
    w: &'a mut W,
    boundary: String,
    lastpart: bool,
}

impl<'a, W> Writer<'a, W>
where
    W: io::Writer,
{
    /// New returns a new multipart Writer with a random boundary, writing to w.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// New函数返回一个设定了一个随机边界的Writer，数据写入w。
    /// </details>
    ///
    /// # Example
    ///
    /// ```
    /// use gostd::bytes;
    /// use gostd::mime::multipart::Writer;
    ///
    ///     let mut body = bytes::Buffer::new();
    ///     let mut w = Writer::new(&mut body);
    ///
    /// ```
    pub fn new(writer: &mut W) -> Writer<W> {
        Writer {
            w: writer,
            boundary: randomBoundary(),
            lastpart: false,
        }
    }
    /// Boundary returns the Writer's boundary.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// 返回该Writer的边界。
    /// </details>
    ///
    pub fn Boundary(&self) -> &str {
        &self.boundary
    }

    /// FormDataContentType returns the Content-Type for an HTTP multipart/form-data with this Writer's Boundary.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// 返回w对应的HTTP multipart请求的Content-Type的值，多以multipart/form-data起始。
    /// </details>
    ///
    pub fn FormDataContentType(&self) -> String {
        let mut b = "".to_string();
        if strings::ContainsAny(self.boundary.clone().as_str(), "()<>@,;:\"/[]?=") {
            b.push('"');
            b.push_str(self.boundary.clone().as_str());
            b.push('"')
        } else {
            b.push_str(self.boundary.clone().as_str());
        }
        format!("multipart/form-data; boundary={}", b)
    }

    /// CreatePart creates a new multipart section with the provided header. The body of the part should be written to the returned Writer. After calling CreatePart, any previous part may no longer be written to.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// CreatePart方法使用提供的header创建一个新的multipart记录。该记录的主体应该写入返回的Writer接口。调用本方法后，任何之前的记录都不能再写入
    /// </details>
    ///
    pub fn CreatePart(&mut self, header: HashMap<String, Vec<String>>) -> Result<&mut W, Error> {
        if self.lastpart {
            return Err(Error::new(std::io::ErrorKind::Other, "is closed"));
        }
        let mut b = bytes::Buffer::new();
        if !self.lastpart {
            b.WriteString(format!("\r\n--{}\r\n", self.boundary.clone()).as_str());
        } else {
            b.WriteString(format!("--{}\r\n", self.boundary.clone()).as_str());
        }
        let mut keys: Vec<String> = Vec::with_capacity(len!(header));
        for k in header.keys() {
            keys.push(k.to_owned());
        }
        keys.sort();
        for k in keys {
            for v in header.get(&k).unwrap() {
                b.WriteString(format!("{}: {}\r\n", k, v).as_str());
            }
        }
        b.WriteString("\r\n");
        self.w.Write(b.Bytes());

        Ok(self.w.borrow_mut())
    }

    /// CreateFormFile is a convenience wrapper around CreatePart. It creates a new form-data header with the provided field name and file name.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// CreateFormFile是CreatePart方法的包装， 使用给出的属性名和文件名创建一个新的form-data头。
    /// </details>
    ///
    pub fn CreateFormFile(&mut self, fieldname: &str, filename: &str) -> Result<&mut W, Error> {
        let mut h: HashMap<String, Vec<String>> = HashMap::new();
        h.insert(
            "Content-Disposition".to_string(),
            vec![format!(
                r#"form-data; name="{}"; filename="{}""#,
                escapeQuotes(fieldname),
                escapeQuotes(filename)
            )],
        );
        h.insert(
            "Content-Type".to_string(),
            vec!["application/octet-stream".to_string()],
        );
        self.CreatePart(h)
    }

    /// CreateFormField calls CreatePart with a header using the given field name.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// CreateFormField方法使用给出的属性名调用CreatePart方法。
    /// </details>
    ///
    pub fn CreateFormField(&mut self, fieldname: &str) -> Result<&mut W, Error> {
        let mut h: HashMap<String, Vec<String>> = HashMap::new();
        h.insert(
            "Content-Disposition".to_string(),
            vec![format!(r#"form-data; name="{}""#, escapeQuotes(fieldname))],
        );
        self.CreatePart(h)
    }

    /// WriteField calls CreateFormField and then writes the given value
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// WriteField方法调用CreateFormField并写入给出的value。
    /// </details>
    ///
    pub fn WriteField(&mut self, fieldname: &str, value: &str) -> Result<(), Error> {
        let mut p = self.CreateFormField(fieldname)?;
        match p.Write(value.as_bytes().to_vec()) {
            Err(err) => Err(err),
            Ok(_) => Ok(()),
        }
    }

    /// Close finishes the multipart message and writes the trailing boundary end line to the output.
    /// <details class="rustdoc-toggle top-doc">
    /// <summary class="docblock">zh-cn</summary>
    /// Close方法结束multipart信息，并将结尾的边界写入底层io.Writer接口。
    /// </details>
    ///
    pub fn Close(&mut self) -> Result<(), Error> {
        if self.lastpart {
            return Err(Error::new(std::io::ErrorKind::Other, "is closed"));
        }
        self.lastpart = true;
        let bound = format!("\r\n--{}--\r\n", self.boundary);
        match self.w.Write(bound.as_bytes().to_vec()) {
            Err(err) => return Err(err),
            Ok(n) => return Ok(()),
            _ => Ok(()),
        }
    }
}

fn escapeQuotes(s: &str) -> String {
    let p = vec![("\\", "\\\\"), (r#"""#, r#"\\\""#)];
    let r = strings::Replacer::new(p);
    r.Replace(s)
}

fn randomBoundary() -> String {
    let mut bytes = [0; 30];
    rand::thread_rng().fill_bytes(&mut bytes);

    fn as_u32(slice: &[u8]) -> u32 {
        let mut copy = [0; 4];
        copy.copy_from_slice(slice);
        u32::from_ne_bytes(copy)
    }

    let a = as_u32(&bytes[0..4]);
    let b = as_u32(&bytes[4..8]);
    let c = as_u32(&bytes[8..12]);
    let d = as_u32(&bytes[12..16]);
    let e = as_u32(&bytes[16..20]);
    let f = as_u32(&bytes[20..24]);
    let g = as_u32(&bytes[24..28]);
    format!("{:x}{:x}{:x}{:x}{:x}{:x}{:x}xx", a, b, c, d, e, f, g)
}