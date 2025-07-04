use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::time::Duration;

use anyhow::Result;
use arguably::ArgParser;

const MD_URL: &str =
  "https://github.com/codecrafters-io/build-your-own-x/raw/refs/heads/master/README.md";

use markdown::{mdast::Node, ParseOptions};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
enum RollError {
  #[error("MD data request failed")]
  Fetch,

  #[error("MD parse failed")]
  Parse,
}

async fn download_md() -> Result<String, RollError> {
  reqwest::get(MD_URL)
    .await
    .or(Err(RollError::Fetch))?
    .text()
    .await
    .or(Err(RollError::Fetch))
}

/// Roll a dn where n -> # of sides on die
/// e.g. roll_die(6) rolls a d6
fn roll_die<S: AsRef<str> + fmt::Display>(n: usize, msg: S, hide_spinner: bool) -> usize {
  use nanorand::{Rng, WyRand};
  use spinners::{Spinner, Spinners};
  let mut rng = WyRand::new();

  if !hide_spinner {
    let mut spin = Spinner::new(Spinners::Dots, format!("{msg} (d{n})"));
    std::thread::sleep(Duration::from_millis(500)); // fake delay, just for fun
    spin.stop();
  }

  rng.generate_range(0..n)
}

#[tokio::main]
async fn main() -> Result<()> {
  let mut parser = ArgParser::new()
    .helptext("Usage: rollthetech ...")
    .version("0.1")
    .flag("fast f");

  if let Err(err) = parser.parse() {
    err.exit();
  }

  let md_text = download_md().await?;
  let ast = markdown::to_mdast(&md_text, &ParseOptions::default()).or(Err(RollError::Parse))?;

  let mut categories: HashMap<String, Vec<String>> = HashMap::new();
  if let Node::Root(root) = ast {
    let mut current_category: Option<String> = None;
    for child in &root.children {
      match child {
        Node::Heading(h) if h.depth == 2 => {
          if let Some(Node::Text(t)) = h.children.first()
            && t.value.starts_with("Contribute")
          {
            break;
          }
        }
        Node::Heading(h) if h.depth == 4 => {
          let Some(Node::Text(t)) = h.children.first() else {
            println!("[!] expected direct text w/ depth 4");
            Err(RollError::Parse)?
          };

          if t.value.starts_with("Build") {
            if let Node::InlineCode(ic) = &h.children[1] {
              current_category = Some(ic.value.clone());
              categories.insert(ic.value.clone(), vec![]);
            } else {
              println!("[!] expected inline code in heading w/ depth 4");
              Err(RollError::Parse)?
            }
          }
        }
        Node::List(l) => {
          if let Some(cc) = &current_category
            && !cc.is_empty()
          {
            for item in &l.children {
              let Node::Link(lnk) = item
                .children()
                .unwrap()
                .first()
                .unwrap() // Paragraph
                .children()
                .unwrap()
                .first()
                .unwrap()
              // Link
              else {
                println!("[!] expected link for category item");
                Err(RollError::Parse)?
              };

              let mut link_title = String::new();
              for link_child in &lnk.children {
                match link_child {
                  Node::Strong(s) => {
                    let Node::Text(stxt) = &s.children[0] else {
                      Err(RollError::Parse)?
                    };
                    write!(&mut link_title, "{{blue}}{{bold}}{}{{-}}: ", stxt.value)?;
                  }
                  Node::Emphasis(e) => {
                    let Node::Text(etxt) = &e.children[0] else {
                      Err(RollError::Parse)?
                    };
                    write!(&mut link_title, "{{white}}{{italic}}{}{{-}}", etxt.value)?;
                  }
                  _ => {}
                }
              }
              categories
                .get_mut(current_category.as_ref().unwrap())
                .unwrap()
                .push(link_title);
            }
          }
        }
        _ => {}
      }
    }
  }

  let category_idx = roll_die(
    categories.keys().len(),
    "Deciding a category... ",
    parser.found("fast"),
  );
  let category = categories.keys().nth(category_idx).unwrap().as_str();
  if parser.found("fast") {
    println!(
      "{}",
      tempera::colorize_template(&format!(" → {{bold}}{{italic}}{category}{{-}}"))
    )
  };

  let projects = &categories[category];
  let project_idx = roll_die(
    projects.len(),
    "Deciding a project...",
    parser.found("fast"),
  );
  if parser.found("fast") {
    println!();
  }

  println!("{}", tempera::colorize_template(&projects[project_idx]));

  Ok(())
}
