//! `presiterm new`: a deck folder to start from.

use std::path::Path;

use anyhow::{Context, Result, ensure};

const SLIDES: &str = "---
title: \"@TITLE@\"
author: \"\"
---

# @TITLE@

**One sentence that carries the talk**

Event, date

---

# Today

- understand **where** something happens
+ a point that appears on a key press
+ and one more

Note: only the presenter sees this (key n).

---

# The chain

```mermaid steps
flowchart LR
  source[Source] --> collector[Collector] --> store[(Store)] --> view[View]
```

:::{notes}
Each key press lights one node.
:::

---

# Code

```bash
journalctl --since -10m -f -u my-app
```

| Layer   | Tool       |
|:--------|-----------:|
| Logs    | journald   |
| Metrics | Prometheus |

---

# From a file

![Flow](assets/flow.mmd){h=8}

---

# Questions
";

const FLOW: &str = "flowchart LR
  a[One] --> b[Two] --> c[Three]
";

/// Create `dir/slides.md` and `dir/assets/flow.mmd`, and return the line to tell the user.
pub fn run(dir: &Path) -> Result<String> {
    ensure!(!dir.exists(), "{} exists already", dir.display());
    std::fs::create_dir_all(dir.join("assets"))
        .with_context(|| format!("create {}", dir.display()))?;
    let title = dir
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(|| dir.to_string_lossy().into_owned(), str::to_string);
    let slides = dir.join("slides.md");
    std::fs::write(&slides, SLIDES.replace("@TITLE@", &title))
        .with_context(|| format!("write {}", slides.display()))?;
    let flow = dir.join("assets/flow.mmd");
    std::fs::write(&flow, FLOW).with_context(|| format!("write {}", flow.display()))?;
    Ok(format!(
        "made {}; next: presiterm --window {}",
        dir.display(),
        slides.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck;

    #[test]
    fn the_scaffold_parses_without_errors() {
        let root = tempfile::tempdir().expect("temp dir");
        let dir = root.path().join("talk");
        let message = run(&dir).expect("scaffold");
        assert_eq!(
            message,
            format!(
                "made {}; next: presiterm --window {}",
                dir.display(),
                dir.join("slides.md").display()
            )
        );
        let slides = dir.join("slides.md");
        let source = std::fs::read_to_string(&slides).expect("slides.md");
        let deck = deck::parse(&source, "t", &dir, false);
        assert!(deck.slides.len() >= 6);
        assert!(
            deck.slides
                .iter()
                .flat_map(|s| &s.blocks)
                .all(|b| !matches!(b, deck::Block::Error(_)))
        );
    }
}
