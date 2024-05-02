use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::string::String;

use html5ever::tendril::TendrilSink;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, Node, NodeData, RcDom};
use html5ever::interface::Attribute;
use html2md::parse_html as to_md;
use html5ever::serialize::{SerializeOpts, serialize};

pub fn parse_html(source: &str) -> RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut source.as_bytes())
        .unwrap()
}

pub fn extract_contents(url: String, handle: Handle) -> () {
    
    let anchor_tags = vec![];
    let mut filter = HashMap::new();
    filter.insert("id".to_owned(), "content".to_owned());
    let mut store = Store::Node(anchor_tags);
    get_elements_by_name(handle, "div", &mut store, Some(filter));
    if let Store::Node(elts) = store {
        for n in elts {
            let mut bytes = vec![];
            serialize(&mut bytes, &n, SerializeOpts::default()).unwrap();
            let result = String::from_utf8(bytes).unwrap();
            let markdown = to_md(&result);
            fs::write(format!("resources/{}", url.replace("/", "_")), markdown).expect("Unable to write file");
        }
    }
}

pub fn get_urls(handle: Handle) -> Vec<String> {

    let mut urls = vec![];
    let anchor_tags = vec![];
    let mut store = Store::Data(anchor_tags);
    
    get_elements_by_name(handle, "a", &mut store, None);
    
    if let Store::Data(elts) = store {
        for node in elts {
            if let NodeData::Element { ref attrs, .. } = node {
                for attr in attrs.borrow().iter() {
                    let Attribute {
                        ref name,
                        ref value,
                    } = *attr;
                    if &*(name.local) == "href" {
                        let mut url_string = value.to_string();
                        if url_string.contains("#") {
                            url_string = url_string
                                .split("#")
                                .nth(0)
                                .unwrap_or("")
                                .to_string();
                        }
                        if !url_string.contains("https://www.famnit.upr.si/en") {
                            let splitter = if url_string.starts_with("/") {
                                ""
                            } else {
                                "/"
                            };
                            url_string = format!("https://www.famnit.upr.si{}{}", splitter, url_string)
                        }
                        urls.push(url_string);
                    }
                }
            }
        }
    }

    urls
}

enum Store {
    Data(Vec<NodeData>),
    Node(Vec<Rc<Node>>)
}

fn get_elements_by_name(
    handle: Handle,
    element_name: &str,
    out: &mut Store,
    attr_filter: Option<HashMap<String, String>>,
) {
    let node = handle;

    if let NodeData::Element {
        ref name,
        ref attrs,
        ref template_contents,
        ..
    } = node.data
    {
        if &*(name.local) == element_name {
            // Additional check for attribute filters
            let mut is_attr_match = true; // Assume true, will be falsified if any check fails
            
            if let Some(ref filter) = attr_filter {
                for (key, val) in filter {
                    let attrs_ref = attrs.borrow();
                    if !attrs_ref.iter().any(|attr| {
                        &*(attr.name.local) == key && *attr.value == *val
                    }) {
                        is_attr_match = false;
                        break; // No need to check further if any attribute doesn't match
                    }
                }
            }

            if is_attr_match {
                match out {
                    Store::Data(out_store) => out_store.push(NodeData::Element {
                        name: name.clone(),
                        attrs: attrs.clone(),
                        template_contents: template_contents.clone(),
                        mathml_annotation_xml_integration_point: false,
                    }),
                    Store::Node(out_store) => out_store.push(node.clone()),
                }
            }
        }
    }

    for n in node.children.borrow().iter() {
        get_elements_by_name(n.clone(), element_name, out, attr_filter.clone()); // Pass attr_filter down recursively
    }
}