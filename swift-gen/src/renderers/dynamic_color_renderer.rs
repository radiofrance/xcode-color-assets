use super::data::*;
use super::renderer::{Renderer, RendererConfig};
use std::collections::HashMap;
use std::rc::Rc;

pub struct DynamicColorRenderer {}

impl Renderer for DynamicColorRenderer {
  fn render_into(&self, ruleset: &RuleSet, d: &mut String, config: &RendererConfig) {
    let mut colorset_map = ColorSetMap::new();
    DynamicColorRenderer::populate_colorset_map(ruleset, &mut colorset_map);

    d.push_str(
      r#"// This file is automatically generated. Do not edit, your changes will be erased.

import UIKit

fileprivate struct ColorSet {
  var light: UIColor
  var dark: UIColor?

  init(_ light: UIColor, _ dark: UIColor?) {
    self.light = light
    self.dark = dark
  }
}

fileprivate func dynamicColor(_ colorSet: ColorSet) -> UIColor {
  if #available(iOS 13.0, *) {
    return UIColor { traits -> UIColor in
      switch traits.userInterfaceStyle {
        case .dark:
          return colorSet.dark ?? colorSet.light
        case .light, .unspecified:
          fallthrough
        @unknown default:
          return colorSet.light
      }
    }
  } else {
    return colorSet.light
  }
}
"#,
    );
    d.push_str("\n");
    d.push_str("fileprivate let ColorSets: [ColorSet] = [\n");

    for colorset in &colorset_map.colorsets {
      d.push_str(&format!(
        "{}ColorSet({}, {}),\n",
        config.indent(1),
        colorset.light.ui_color_string(),
        colorset
          .dark
          .map_or("nil".to_string(), |color| color.ui_color_string())
      ))
    }

    d.push_str("]\n");
    d.push_str("\n");
    d.push_str("extension UIColor {\n");
    self.render_ruleset_into(ruleset, d, &colorset_map, config);
    d.push_str("}\n");
  }
}

#[derive(Hash, PartialEq, Eq)]
struct CompatColorSet<'a> {
  light: &'a Color,
  dark: Option<&'a Color>,
}

impl<'a> From<&'a Declaration> for CompatColorSet<'a> {
  fn from(decl: &'a Declaration) -> Self {
    match &decl.value {
      DeclarationValue::Color(color) => CompatColorSet {
        light: color,
        dark: None,
      },
      DeclarationValue::ColorSet(colorset) => CompatColorSet {
        light: &colorset.light,
        dark: Some(&colorset.dark),
      },
    }
  }
}

struct ColorSetMap<'a> {
  map: HashMap<Rc<CompatColorSet<'a>>, usize>,
  colorsets: Vec<Rc<CompatColorSet<'a>>>,
}

impl Color {
  fn ui_color_string(&self) -> String {
    format!(
      "UIColor(red: {:.3}, green: {:.3}, blue: {:.3}, alpha: {:.2})",
      f32::from(self.r) / 255.0,
      f32::from(self.g) / 255.0,
      f32::from(self.b) / 255.0,
      self.a
    )
  }
}

impl<'a> ColorSetMap<'a> {
  fn new() -> Self {
    ColorSetMap {
      map: HashMap::new(),
      colorsets: vec![],
    }
  }

  fn register_declaration(&mut self, decl: &'a Declaration) {
    let colorset = Rc::new(CompatColorSet::from(decl));

    if self.map.get(&colorset).is_some() {
      return;
    }

    let idx = self.colorsets.len();
    self.colorsets.push(colorset.clone());
    self.map.insert(colorset, idx);
  }

  fn index_for_declaration(&self, decl: &'a Declaration) -> &usize {
    let colorset = CompatColorSet::from(decl);
    self
      .map
      .get(&colorset)
      .expect("Could not get index for declaration.")
  }
}

impl DynamicColorRenderer {
  fn populate_colorset_map<'a>(ruleset: &'a RuleSet, map: &mut ColorSetMap<'a>) {
    for item in &ruleset.items {
      match item {
        RuleSetItem::Declaration(decl) => map.register_declaration(&decl),
        RuleSetItem::RuleSet(ruleset) => Self::populate_colorset_map(&ruleset, map),
      }
    }
  }

  fn render_ruleset_into(
    &self,
    ruleset: &RuleSet,
    d: &mut String,
    map: &ColorSetMap,
    config: &RendererConfig,
  ) {
    d.push_str(&format!(
      "{}enum {} {{\n",
      config.indent(ruleset.identifier.depth),
      ruleset.identifier.short
    ));

    for item in &ruleset.items {
      match item {
        RuleSetItem::Declaration(decl) => self.render_declaration_into(decl, d, map, config),
        RuleSetItem::RuleSet(ruleset) => self.render_ruleset_into(ruleset, d, map, config),
      }
    }

    d.push_str(&format!("{}}}\n", config.indent(ruleset.identifier.depth)));
  }

  fn render_declaration_into(
    &self,
    declaration: &Declaration,
    d: &mut String,
    map: &ColorSetMap,
    config: &RendererConfig,
  ) {
    d.push_str(&format!(
      "{}static let {} = dynamicColor(ColorSets[{}])\n",
      config.indent(declaration.identifier.depth),
      declaration.identifier.short,
      map.index_for_declaration(declaration)
    ))
  }
}
