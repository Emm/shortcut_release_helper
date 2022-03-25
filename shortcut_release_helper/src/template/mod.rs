mod utils;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::offset::Utc;
use minijinja::{
    value::{Value, ValueKind},
    Environment, ErrorKind, State,
};

use crate::Release;
use utils::SeqIterator;

#[derive(Debug)]
pub struct FileTemplate<'a> {
    environment: Environment<'a>,
}

const TEMPLATE_NAME: &'static str = "main";

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

        environment.add_function("today", Self::today);

        Ok(Self { environment })
    }

    fn split_by_label<'s>(
        _state: &State,
        v: Value,
        label: Value,
    ) -> Result<Value, minijinja::Error> {
        let label_name = if matches!(label.kind(), ValueKind::String) {
            label.as_str().expect("Should be a string")
        } else {
            return Err(minijinja::Error::new(
                ErrorKind::ImpossibleOperation,
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

    fn split_by_epic_stories_state<'s>(
        _state: &State,
        v: Value,
    ) -> Result<Value, minijinja::Error> {
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

    fn split_by_epic<'s>(
        _state: &State,
        v: Value,
        epic_id: Value,
    ) -> Result<Value, minijinja::Error> {
        if !matches!(epic_id.kind(), ValueKind::Number) {
            return Err(minijinja::Error::new(
                ErrorKind::ImpossibleOperation,
                "expected a number",
            ));
        };
        let (mut matched, mut unmatched) = (Vec::new(), Vec::new());
        let stories_iter = SeqIterator::new(v)?;
        for story in stories_iter {
            if story.get_attr("epic_id")? == epic_id {
                matched.push(story)
            } else {
                unmatched.push(story)
            };
        }
        Ok(Value::from(vec![matched, unmatched]))
    }

    /// Helper returning today's date, formatted according to a format string following
    /// [`chrono::format::strftime`] (if present), otherwise defaults to `YYYY-MM-DD`.
    fn today<'s>(_state: &State, fmt: Option<String>) -> Result<Value, minijinja::Error> {
        Ok(Value::from_safe_string(
            Utc::today()
                .format(fmt.as_deref().unwrap_or("%F"))
                .to_string(),
        ))
    }

    pub fn render_to_file(&self, release: &Release, output_file: &PathBuf) -> Result<()> {
        let template = self.environment.get_template(TEMPLATE_NAME)?;
        let file_content = template.render(&release)?;
        fs::write(output_file, &file_content)?;
        Ok(())
    }
}
