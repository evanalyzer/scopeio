//! # meta
//!
//! Core metadata from the CZI XML (`ImageDocument/Metadata`): pixel
//! sizes, channels, objective, scene names, acquisition date.
//!
//! Provenance: OpenReadout CZI format notes (MIT OR Apache-2.0) in
//! `doc/zeiss-czi/`; own analysis of sample files. No GPL/LGPL code
//! consulted.
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

use std::collections::HashMap;

use roxmltree::{Document, Node};

use crate::{ChannelMeta, ObjectiveMeta};

/// Fields of the XML used for the core metadata.
#[derive(Debug, Clone, Default)]
pub(super) struct XmlMeta {
    /// Pixel size of X, Y, Z in nanometres.
    pub scale_nm: [Option<f64>; 3],
    pub channels: Vec<ChannelMeta>,
    pub significant_bits: Option<u8>,
    pub date: Option<String>,
    pub objective: Option<ObjectiveMeta>,
    /// Scene names by scene index.
    pub scenes: HashMap<i32, String>,
    pub name: Option<String>,
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(name))
}

fn path<'a, 'i>(n: Node<'a, 'i>, names: &[&str]) -> Option<Node<'a, 'i>> {
    names.iter().try_fold(n, |n, name| child(n, name))
}

fn text(n: Option<Node>) -> Option<String> {
    n.and_then(|n| n.text()).map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

fn number(n: Option<Node>) -> Option<f64> {
    text(n)?.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// `#AARRGGBB` (or `#RRGGBB`) as RGBA with full alpha.
fn color(s: &str) -> Option<[u8; 4]> {
    let hex = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 | 8 => Some([(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]),
        _ => None,
    }
}

/// Parses the XML; malformed XML yields empty metadata.
pub(super) fn parse(xml: &str) -> XmlMeta {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let Ok(doc) = Document::parse_with_options(xml, options) else { return XmlMeta::default() };
    let root = doc.root_element();
    let Some(meta) = child(root, "Metadata") else { return XmlMeta::default() };
    let mut out = XmlMeta::default();

    if let Some(items) = path(meta, &["Scaling", "Items"]) {
        for d in items.children().filter(|c| c.has_tag_name("Distance")) {
            let slot = match d.attribute("Id") {
                Some("X") => 0,
                Some("Y") => 1,
                Some("Z") => 2,
                _ => continue,
            };
            // Metres per pixel.
            out.scale_nm[slot] = number(child(d, "Value")).filter(|v| *v > 0.0).map(|v| v * 1e9);
        }
    }

    let info = child(meta, "Information");
    let image = info.and_then(|i| child(i, "Image"));
    if let Some(image) = image {
        out.significant_bits = number(child(image, "ComponentBitCount")).map(|v| v as u8).filter(|&b| b > 0);
        out.date = text(child(image, "AcquisitionDateAndTime"));
        if let Some(scenes) = path(image, &["Dimensions", "S", "Scenes"]) {
            for s in scenes.children().filter(|c| c.has_tag_name("Scene")) {
                if let (Some(i), Some(name)) = (s.attribute("Index").and_then(|i| i.parse().ok()), s.attribute("Name")) {
                    out.scenes.insert(i, name.to_string());
                }
            }
        }
    }
    out.name = info.and_then(|i| text(path(i, &["Document", "Name"])));

    // Display settings complete the image channels, matched by id.
    let display: Vec<Node> = path(meta, &["DisplaySetting", "Channels"])
        .map(|c| c.children().filter(|c| c.has_tag_name("Channel")).collect())
        .unwrap_or_default();
    let image_channels: Vec<Node> = image
        .and_then(|i| path(i, &["Dimensions", "Channels"]))
        .map(|c| c.children().filter(|c| c.has_tag_name("Channel")).collect())
        .unwrap_or_default();
    let list = if image_channels.is_empty() { &display } else { &image_channels };
    for (i, ch) in list.iter().enumerate() {
        let disp = display.iter().find(|d| d.attribute("Id").is_some() && d.attribute("Id") == ch.attribute("Id")).or(display.get(i));
        let field = |name: &str| text(child(*ch, name)).or_else(|| disp.and_then(|d| text(child(*d, name))));
        let wavelength = |name: &str| field(name).and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0);
        out.channels.push(ChannelMeta {
            name: ch.attribute("Name").filter(|n| !n.is_empty()).map(String::from).or_else(|| field("DyeName")),
            color: field("Color").as_deref().and_then(color),
            excitation_wavelength_nm: wavelength("ExcitationWavelength"),
            emission_wavelength_nm: wavelength("EmissionWavelength"),
            fluor: field("Fluor").or_else(|| field("DyeName")),
            contrast_method: field("ContrastMethod"),
            // Nanoseconds.
            exposure_s: wavelength("ExposureTime").map(|ns| ns / 1e9),
            ..Default::default()
        });
    }

    if let Some(o) = info.and_then(|i| path(i, &["Instrument", "Objectives"])).and_then(|o| child(o, "Objective")) {
        let model = text(child(o, "Model")).or_else(|| o.attribute("Name").filter(|n| !n.is_empty()).map(String::from));
        out.objective = Some(ObjectiveMeta {
            manufacturer: text(path(o, &["Manufacturer", "Model"])).or_else(|| text(child(o, "Manufacturer"))),
            model,
            magnification: number(child(o, "NominalMagnification")).filter(|v| *v > 0.0),
            numerical_aperture: number(child(o, "LensNA")).filter(|v| *v > 0.0),
            immersion: text(child(o, "Immersion")),
        });
    }
    out
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_fields() {
        let xml = r##"<ImageDocument><Metadata>
            <Information><Image><ComponentBitCount>14</ComponentBitCount>
              <AcquisitionDateAndTime>2022-08-22T07:08:44Z</AcquisitionDateAndTime>
              <Dimensions><Channels><Channel Id="Channel:0" Name="EGFP"><ExcitationWavelength>488</ExcitationWavelength>
                <ExposureTime>10000000</ExposureTime></Channel></Channels>
              <S><Scenes><Scene Index="1" Name="B3"/></Scenes></S></Dimensions></Image>
              <Instrument><Objectives><Objective Name="Plan 5x"><LensNA>0.35</LensNA><NominalMagnification>5</NominalMagnification></Objective></Objectives></Instrument>
            </Information>
            <Scaling><Items><Distance Id="X"><Value>1E-07</Value></Distance><Distance Id="Z"><Value>2.5E-06</Value></Distance></Items></Scaling>
            <DisplaySetting><Channels><Channel Id="Channel:0"><Color>#FF00FF5B</Color></Channel></Channels></DisplaySetting>
        </Metadata></ImageDocument>"##;
        let m = parse(xml);
        assert_eq!(m.scale_nm[0], Some(100.0));
        assert!((m.scale_nm[2].unwrap() - 2500.0).abs() < 1e-9);
        assert_eq!(m.significant_bits, Some(14));
        let c = &m.channels[0];
        assert_eq!((c.name.as_deref(), c.color, c.excitation_wavelength_nm), (Some("EGFP"), Some([0, 0xFF, 0x5B, 255]), Some(488.0)));
        assert_eq!(c.exposure_s, Some(0.01));
        assert_eq!(m.scenes.get(&1).map(String::as_str), Some("B3"));
        let o = m.objective.unwrap();
        assert_eq!((o.model.as_deref(), o.magnification, o.numerical_aperture), (Some("Plan 5x"), Some(5.0), Some(0.35)));
    }
}
