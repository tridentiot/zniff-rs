// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT

//! XML output module for serializing parsed Z-Wave frames to XML format.
//!
//! This module provides structures and functions to convert parsed Z-Wave frames
//! into a well-structured XML format suitable for analysis and processing.

use std::io::Write;

/// Represents a single parsed field from a Z-Wave frame.
#[derive(Debug, Clone)]
pub struct ParsedField {
    pub name: String,
    pub value: String,
    pub field_type: Option<String>,
}

/// Represents a complete parsed Z-Wave frame with all its fields.
#[derive(Debug, Clone, Default)]
pub struct ParsedFrame {
    pub fields: Vec<ParsedField>,
}

impl ParsedFrame {
    /// Creates a new empty ParsedFrame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field to the frame with the given name, value, and optional type.
    pub fn add_field(&mut self, name: String, value: String, field_type: Option<String>) {
        self.fields.push(ParsedField {
            name,
            value,
            field_type,
        });
    }
}

/// XML writer for converting parsed frames to XML format.
#[derive(Default)]
pub struct XmlWriter {
    frames: Vec<ParsedFrame>,
}

impl XmlWriter {
    /// Creates a new empty XmlWriter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a parsed frame to be written to the XML output.
    pub fn add_frame(&mut self, frame: ParsedFrame) {
        self.frames.push(frame);
    }

    /// Writes all frames to an XML file at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path where the XML should be written
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an `std::io::Error` if writing fails.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_frame_creation() {
        let mut frame = ParsedFrame::new();
        frame.add_field("TestField".to_string(), "TestValue".to_string(), Some("string".to_string()));
        
        assert_eq!(frame.fields.len(), 1);
        assert_eq!(frame.fields[0].name, "TestField");
        assert_eq!(frame.fields[0].value, "TestValue");
        assert_eq!(frame.fields[0].field_type, Some("string".to_string()));
    }

    #[test]
    fn test_xml_writer() {
        let mut writer = XmlWriter::new();
        let mut frame = ParsedFrame::new();
        frame.add_field("Field1".to_string(), "Value1".to_string(), Some("type1".to_string()));
        frame.add_field("Field2".to_string(), "Value2".to_string(), None);
        
        writer.add_frame(frame);
        
        assert_eq!(writer.frames.len(), 1);
        assert_eq!(writer.frames[0].fields.len(), 2);
    }

    #[test]
    fn test_xml_escaping() {
        let input = "<tag>value & \"quoted\"</tag>";
        let expected = "&lt;tag&gt;value &amp; &quot;quoted&quot;&lt;/tag&gt;";
        assert_eq!(escape_xml(input), expected);
    }

    #[test]
    fn test_xml_output_format() {
        let mut writer = XmlWriter::new();
        let mut frame = ParsedFrame::new();
        frame.add_field("TestName".to_string(), "TestValue".to_string(), Some("TestType".to_string()));
        
        writer.add_frame(frame);
        
        // Write to a temporary file
        let temp_path = "/tmp/test_xml_output.xml";
        writer.write_to_file(temp_path).expect("Failed to write XML");
        
        // Read back and verify content
        let content = std::fs::read_to_string(temp_path).expect("Failed to read XML");
        assert!(content.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(content.contains("<frames>"));
        assert!(content.contains("<frame index=\"0\">"));
        assert!(content.contains("<field name=\"TestName\" type=\"TestType\">TestValue</field>"));
        assert!(content.contains("</frame>"));
        assert!(content.contains("</frames>"));
        
        // Clean up
        std::fs::remove_file(temp_path).ok();
    }
}
