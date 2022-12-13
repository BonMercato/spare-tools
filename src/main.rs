use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{fmt::Write, io::{Write as IoWrite, Read}};

#[cfg(windows)] pub const NL: &str = "\r\n";
#[cfg(not(windows))] pub const NL: &str = "\n";

fn does_exist(path: &str) -> Result<String, String> {
    if std::path::Path::new(path).exists() {
        Ok(path.to_string())
    } else {
        Err(format!("\"{}\" does not exist", path))
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = "")]
struct Args {
    /// Reads the input file as ANSI instead of UTF-8
    #[clap(short, long)]
    pub ansi: bool,

    #[clap(value_parser = does_exist)]
    pub input: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Product {
    #[serde(rename(serialize = "$unflatten=id", deserialize = "LFDNR"))]
    pub id: u32,
    #[serde(rename(serialize = "$unflatten=articleNumber", deserialize = "ART_ID_ET"))]
    pub article_number: String,
    #[serde(rename(serialize = "$unflatten=articleDescription", deserialize = "DESC_ET"))]
    pub article_description: String,
    #[serde(rename(serialize = "$unflatten=orderNumber", deserialize = "BESTELLNUMMER"))]
    pub order_number: String,
    #[serde(rename(serialize = "$unflatten=orderDescription", deserialize = "BESTELLTEXT"))]
    pub order_description: String,
    #[serde(rename(serialize = "$unflatten=articleSearchText", deserialize = "SUCHTEXT"))]
    pub article_search_text: String,
}

#[derive(Serialize, Debug)]
#[serde(rename = "productList")]
struct ProductList {
    #[serde(rename = "product")]
    pub products: Vec<Product>,
}

// append xml header
fn append_xml_header(xml: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>{}{}",
        NL, xml
    )
}

/// Pretty-prints the given XML.
pub fn prettify_xml(xml: &str) -> String {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.trim_text(true);

    let mut writer = quick_xml::Writer::new_with_indent(Vec::new(), b' ', 4);

    loop {
        let ev = reader.read_event();

        match ev {
            Ok(quick_xml::events::Event::Eof) => break, // exits the loop when reaching end of file
            Ok(event) => writer.write_event(&event),
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
        }
        .expect("Failed to parse XML");
    }

    let result = std::str::from_utf8(&*writer.into_inner())
        .expect("Failed to convert a slice of bytes to a string slice")
        .to_string();

    result
}

pub fn replace_empty_tags(xml: &str) -> String {
    let regex = fancy_regex::Regex::new(r"<([^/]+?)>\s+</[^/]+?>").unwrap();
    let result = regex.replace_all(xml, |caps: &fancy_regex::Captures| {
        format!("<{} />", &caps[1])
    });

    result.to_string()
}

fn main() {
    let args = Args::parse();

    let input_bytes = if args.ansi {
        let mut reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
            .encoding(Some(encoding_rs::WINDOWS_1252))
            .build(std::fs::File::open(&args.input).unwrap());
        let mut input = String::new();
        reader.read_to_string(&mut input).unwrap();
        input.into_bytes()
    } else {
        std::fs::read(&args.input).expect("Failed to read input file")
    };

    let byte_reader = std::io::Cursor::new(input_bytes);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(byte_reader);
    let mut products = Vec::new();
    for result in rdr.deserialize() {
        let mut record: Product = result.unwrap();
        record.article_search_text = format!("<![CDATA[{}]]>", record.article_search_text);
        products.push(record);
    }
    let product_list = ProductList { products };
    let xml = quick_xml::se::to_string(&product_list).unwrap()
        // hack to replace cdata tags with the correct ones
        .replace("&lt;![CDATA[", "<![CDATA[")
        .replace("]]&gt;", "]]>");
    let xml = prettify_xml(&xml);
    let xml = replace_empty_tags(&xml);
    let xml = append_xml_header(&xml);

    // write to SpareParts_Import.xml
    let mut file = std::fs::File::create("SpareParts_Import.xml").unwrap();
    file.write_all(xml.as_bytes()).expect("Failed to write output file");
}
