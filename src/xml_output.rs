// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT

use std::io::Write;

#[derive(Debug, Clone)]
pub struct ParsedField {
    pub name: String,
    pub value: String,
    pub field_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedFrame {
    pub fields: Vec<ParsedField>,
}

impl ParsedFrame {
    pub fn new() -> Self {
        ParsedFrame {
            fields: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: String, value: String, field_type: Option<String>) {
        self.fields.push(ParsedField {
            name,
            value,
            field_type,
        });
    }
}

pub struct XmlWriter {
    frames: Vec<ParsedFrame>,
}

impl XmlWriter {
    pub fn new() -> Self {
        XmlWriter {
            frames: Vec::new(),
        }
    }

    pub fn add_frame(&mut self, frame: ParsedFrame) {
        self.frames.push(frame);
    }

    pub fn write_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        
        writeln!(file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(file, "<frames>")?;
        
        for (index, frame) in self.frames.iter().enumerate() {
            writeln!(file, "  <frame index=\"{}\">", index)?;
            for field in &frame.fields {
                if let Some(ref field_type) = field.field_type {
                    writeln!(
                        file,
                        "    <field name=\"{}\" type=\"{}\">{}</field>",
                        escape_xml(&field.name),
                        escape_xml(field_type),
                        escape_xml(&field.value)
                    )?;
                } else {
                    writeln!(
                        file,
                        "    <field name=\"{}\">{}</field>",
                        escape_xml(&field.name),
                        escape_xml(&field.value)
                    )?;
                }
            }
            writeln!(file, "  </frame>")?;
        }
        
        writeln!(file, "</frames>")?;
        Ok(())
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
