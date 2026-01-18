mod utils;

use std::{fs, path::Path};

use anyhow::Result;
use chrono::offset::Utc;
use lazy_static::lazy_static;
use minijinja::{
    value::{Value, ValueKind},
    Environment, ErrorKind, State,
};
use regex::{Captures, Regex};

use crate::Release;
use utils::SeqIterator;

#[derive(Debug)]
pub struct FileTemplate<'a> {
    environment: Environment<'a>,
}

const TEMPLATE_NAME: &str = "main";

impl<'a> FileTemplate<'a> {
    pub fn new(template_content: &'a str) -> Result<Self> {
        let mut environment = Environment::new();

        environment.add_template(TEMPLATE_NAME, template_content)?;

        environment.add_filter(
            "split_by_epic_stories_state",
            Self::split_by_epic_stories_state,
        );
        environment.add_filter("split_by_label", Self::split_by_label);
        environment.add_filter("split_by_epic", Self::split_by_epic);
        environment.add_filter("has_label", Self::has_label);
        environment.add_filter("story_emoji", Self::story_emoji);
        environment.add_filter("indent", Self::indent);
        environment.add_filter("escape", Self::escape);

        environment.add_function("today", Self::today);
        environment.add_function("epic_emoji", Self::epic_emoji);

        environment.set_auto_escape_callback(|_| minijinja::AutoEscape::None);

        Ok(Self { environment })
    }

    /// Escape Markdown characters - useful for epic and story titles
    fn escape(_state: &State, v: Value) -> Result<Value, minijinja::Error> {
        lazy_static! {
            static ref MARKDOWN_ESCAPE_RE: Regex =
                Regex::new(r##"([!"#$%&'()*+,-./:;<=>?@\[\]^_`{|}~\\])"##)
                    .expect("Markdown escape regex does not compile");
        };
        let v = match v.kind() {
            ValueKind::String => {
                let string = v.as_str().expect("should be a string");
                let escaped_string = MARKDOWN_ESCAPE_RE
                    .replace_all(string, |caps: &Captures| format!(r"\{}", &caps[1]))
                    .to_string();
                Value::from_safe_string(escaped_string)
            }
            _ => v,
        };
        Ok(v)
    }

    /// Indent multiline text by prefixing the platform's linebreak in the value by the amount of
    /// spaces indicated.
    fn indent(_state: &State, v: Value, amount: Value) -> Result<Value, minijinja::Error> {
        #[cfg(windows)]
        const LINE_ENDING: &str = "\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &str = "\n";

        let v = if matches!(v.kind(), ValueKind::String) {
            v.as_str().expect("Should be a string")
        } else {
            return Err(minijinja::Error::new(
                ErrorKind::InvalidOperation,
                "expected a string",
            ));
        };

        let amount = if matches!(amount.kind(), ValueKind::Number) {
            amount.to_string().parse::<usize>().map_err(|err| {
                minijinja::Error::new(
                    ErrorKind::InvalidOperation,
                    format!("could not parse number, got {:?}", err),
                )
            })
        } else {
            Err(minijinja::Error::new(
                ErrorKind::InvalidOperation,
                "expected a number",
            ))
        }?;
        let replacement = format!("{LINE_ENDING} {:amount$}", "");
        let v = v.replace(LINE_ENDING, &replacement);
        Ok(Value::from(v))
    }

    fn split_by_label(_state: &State, v: Value, label: Value) -> Result<Value, minijinja::Error> {
        let label_name = if matches!(label.kind(), ValueKind::String) {
            label.as_str().expect("Should be a string")
        } else {
            return Err(minijinja::Error::new(
                ErrorKind::InvalidOperation,
                "expected a string",
            ));
        };
        let (mut matched, mut unmatched) = (Vec::new(), Vec::new());
        let epic_or_stories_iter = SeqIterator::new(v)?;
        for epic_or_story in epic_or_stories_iter {
            let labels = epic_or_story.get_attr("labels")?;
            let mut labels_iter = SeqIterator::new(labels)?;
            let has_label = labels_iter.any(|label| {
                label.get_attr("name").map_or(false, |name| {
                    name.as_str().map_or(false, |name| name == label_name)
                })
            });
            if has_label {
                matched.push(epic_or_story)
            } else {
                unmatched.push(epic_or_story)
            };
        }
        Ok(Value::from(vec![matched, unmatched]))
    }

    fn split_by_epic_stories_state(_state: &State, v: Value) -> Result<Value, minijinja::Error> {
        let (mut matched, mut unmatched) = (Vec::new(), Vec::new());
        let epics_iter = SeqIterator::new(v)?;
        for epic in epics_iter {
            let stats = epic.get_attr("stats")?;
            let num_stories_total = stats.get_attr("num_stories_total")?;
            let num_stories_done = stats.get_attr("num_stories_done")?;
            let all_done = num_stories_total == num_stories_done;

            if all_done {
                matched.push(epic)
            } else {
                unmatched.push(epic)
            };
        }
        Ok(Value::from(vec![matched, unmatched]))
    }

    fn split_by_epic(
        _state: &State,
        v: Value,
        epic_id: Option<Value>,
    ) -> Result<Value, minijinja::Error> {
        if let Some(epic_id) = epic_id.as_ref() {
            if !matches!(epic_id.kind(), ValueKind::Number) {
                return Err(minijinja::Error::new(
                    ErrorKind::InvalidOperation,
                    "expected a number",
                ));
            }
        };
        let (mut matched, mut unmatched) = (Vec::new(), Vec::new());
        let stories_iter = SeqIterator::new(v)?;
        for story in stories_iter {
            let story_epic_id = story.get_attr("epic_id")?;
            let is_matched = if let Some(epic_id) = epic_id.as_ref() {
                *epic_id == story_epic_id
            } else {
                matches!(story_epic_id.kind(), ValueKind::Number)
            };
            if is_matched {
                matched.push(story)
            } else {
                unmatched.push(story)
            };
        }
        Ok(Value::from(vec![matched, unmatched]))
    }

    fn story_emoji(_state: &State, story: Value) -> Result<Value, minijinja::Error> {
        if !matches!(story.kind(), ValueKind::Map) {
            return Err(minijinja::Error::new(
                ErrorKind::InvalidOperation,
                "expected an object",
            ));
        }
        const FEATURE_EMOJI: &str = ":sunny:";
        const CHORE_EMOJI: &str = ":wrench:";
        const BUG_EMOJI: &str = ":lady_beetle:";
        if let Some(story_type) = story.get_attr("story_type")?.as_str() {
            let emoji = match story_type {
                "feature" => FEATURE_EMOJI,
                "chore" => CHORE_EMOJI,
                "bug" => BUG_EMOJI,
                other => {
                    return Err(minijinja::Error::new(
                        ErrorKind::InvalidOperation,
                        format!("Unknown story_type {}", other),
                    ))
                }
            };
            Ok(Value::from_safe_string(emoji.to_string()))
        } else {
            Err(minijinja::Error::new(
                ErrorKind::InvalidOperation,
                "no story_type attribute",
            ))
        }
    }

    fn has_label(
        _state: &State,
        epic_or_story: Value,
        label: Value,
    ) -> Result<Value, minijinja::Error> {
        let label_name = if matches!(label.kind(), ValueKind::String) {
            label.as_str().expect("Should be a string")
        } else {
            return Err(minijinja::Error::new(
                ErrorKind::InvalidOperation,
                "expected a string",
            ));
        };
        let labels = epic_or_story.get_attr("labels")?;
        let mut labels_iter = SeqIterator::new(labels)?;
        let has_label = labels_iter.any(|label| {
            label.get_attr("name").map_or(false, |name| {
                name.as_str().map_or(false, |name| name == label_name)
            })
        });
        Ok(Value::from(has_label))
    }

    /// Helper returning today's date, formatted according to a format string following
    /// [`chrono::format::strftime`] (if present), otherwise defaults to `YYYY-MM-DD`.
    fn today(_state: &State, fmt: Option<String>) -> Result<Value, minijinja::Error> {
        Ok(Value::from_safe_string(
            Utc::now()
                .format(fmt.as_deref().unwrap_or("%F"))
                .to_string(),
        ))
    }

    fn epic_emoji(_state: &State) -> Result<Value, minijinja::Error> {
        const EPIC_EMOJI: &str = ":checkered_flag:";
        Ok(Value::from_safe_string(EPIC_EMOJI.to_string()))
    }

    pub fn render_to_file(&self, release: &Release, output_file: &Path) -> Result<()> {
        let template = self.environment.get_template(TEMPLATE_NAME)?;
        let file_content = template.render(release)?;
        fs::write(output_file, &file_content)?;
        Ok(())
    }
}
