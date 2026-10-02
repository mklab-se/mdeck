//! The gantt source format and resolving it into dated tasks.

use super::super::VizReveal;
use super::super::grammar::{Problem, Source};
use super::date::{Date, Duration, apply_duration, parse_duration};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) struct GanttTask {
    pub(super) name: String,
    pub(super) start: Option<Date>,
    pub(super) end: Option<Date>,
    pub(super) duration: Option<Duration>,
    /// (task_name, delay_duration)
    pub(super) dependencies: Vec<(String, Option<Duration>)>,
    pub(super) reveal: VizReveal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum LabelMode {
    Side,
    Inside,
}

#[derive(Debug, Clone)]
pub(super) struct GanttData {
    pub(super) tasks: Vec<GanttTask>,
    pub(super) title: Option<String>,
    pub(super) labels: LabelMode,
}

fn read(src: &Source) -> GanttData {
    src.check_settings(&["title", "labels"]);
    let labels = match src.choice("labels", &["side", "inside"]) {
        Some("inside") => LabelMode::Inside,
        _ => LabelMode::Side,
    };
    let mut tasks = Vec::new();
    for item in &src.items {
        item.check_attrs(src, &[]);
        // "Task Name: spec1, spec2, ..."
        let Some((name, specs_str)) = item.label_value() else {
            src.problem(
                item.offset,
                format!(
                    "'{}' is not a task, e.g. '- Design: 5d, after Research'",
                    item.text
                ),
            );
            continue;
        };
        let mut start = None;
        let mut end = None;
        let mut duration = None;
        let mut dependencies = Vec::new();

        for spec in split_specs(specs_str) {
            let spec = spec.trim();
            if spec.is_empty() {
                continue;
            }
            // "after TaskName" or "after TaskName + 3d"
            if let Some(rest) = spec.strip_prefix("after ") {
                let (dep_name, delay) = parse_dependency(rest);
                dependencies.push((dep_name, delay));
            } else if let Some(d) = Date::parse(spec) {
                if start.is_none() {
                    start = Some(d);
                } else {
                    end = Some(d);
                }
            } else if let Some(d) = parse_duration(spec) {
                duration = Some(d);
            } else {
                src.problem(
                    item.offset,
                    format!(
                        "'{spec}' is not a date (2024-01-15), a duration (5d, 3wd, 2w, 1m) or 'after Task'"
                    ),
                );
            }
        }

        tasks.push(GanttTask {
            name: name.to_string(),
            start,
            end,
            duration,
            dependencies,
            reveal: item.reveal,
        });
    }
    for (task, item) in tasks
        .iter()
        .zip(src.items.iter().filter(|i| i.label_value().is_some()))
    {
        for (dep, _) in &task.dependencies {
            if !tasks.iter().any(|t| t.name == *dep) {
                src.problem(
                    item.offset,
                    format!("'after {dep}': there is no task '{dep}'"),
                );
            }
        }
    }

    GanttData {
        tasks,
        title: src.setting("title").map(str::to_string),
        labels,
    }
}

pub(super) fn parse_gantt(content: &str) -> GanttData {
    read(&Source::parse(content))
}

/// The problems in a `@gantt` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

/// Split specs by comma, but respect "after Task + 3d" as a single spec.
pub(super) fn split_specs(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut after_mode = false;

    for part in s.split(", ") {
        if part.starts_with("after ") {
            if !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }
            after_mode = true;
            current = part.to_string();
        } else if after_mode && part.starts_with('+') {
            // This is the delay part of "after Task + 3d"
            current.push_str(", ");
            current.push_str(part);
            after_mode = false;
        } else {
            after_mode = false;
            if !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }
            current = part.to_string();
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

pub(super) fn parse_dependency(rest: &str) -> (String, Option<Duration>) {
    // "TaskName + 3d" or just "TaskName"
    if let Some(plus_pos) = rest.find(" + ") {
        let dep_name = rest[..plus_pos].trim().to_string();
        let delay_str = rest[plus_pos + 3..].trim();
        let delay = parse_duration(delay_str);
        (dep_name, delay)
    } else {
        (rest.trim().to_string(), None)
    }
}

// ─── Resolution ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) struct ResolvedTask {
    pub(super) name: String,
    pub(super) start: Date,
    pub(super) end: Date,
    /// Carried over from the source task so arrows never index by position:
    /// unresolvable tasks are dropped, which shifts every later index.
    pub(super) dependencies: Vec<(String, Option<Duration>)>,
    pub(super) reveal: VizReveal,
}

pub(super) fn resolve_tasks(data: &GanttData) -> Vec<ResolvedTask> {
    let mut resolved: Vec<ResolvedTask> = Vec::new();

    for task in &data.tasks {
        let dep_end = task
            .dependencies
            .iter()
            .filter_map(|(dep_name, delay)| {
                resolved.iter().find(|r| r.name == *dep_name).map(|r| {
                    if let Some(d) = delay {
                        apply_duration(&r.end, *d)
                    } else {
                        r.end
                    }
                })
            })
            .max();

        let (start, end) = match (task.start, task.end, task.duration, dep_end) {
            // Start + End explicit
            (Some(s), Some(e), _, _) => (s, e),
            // Start + Duration
            (Some(s), None, Some(dur), _) => (s, apply_duration(&s, dur)),
            // Duration + End
            (None, Some(e), Some(dur), _) => {
                let dur_days = match dur {
                    Duration::Days(n) => n,
                    Duration::WorkDays(n) => n, // approximate
                };
                (e.add_days(-dur_days), e)
            }
            // Dependency + Duration
            (None, None, Some(dur), Some(dep_e)) => (dep_e, apply_duration(&dep_e, dur)),
            // Dependency only (1 day default)
            (None, None, None, Some(dep_e)) => (dep_e, dep_e.add_days(1)),
            // Start only (default 1 day)
            (Some(s), None, None, _) => (s, s.add_days(1)),
            // No info at all: skip
            // Fallback: use dependency end as start, or skip
            _ => continue,
        };

        resolved.push(ResolvedTask {
            name: task.name.clone(),
            start,
            end,
            dependencies: task.dependencies.clone(),
            reveal: task.reveal,
        });
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gantt_basic() {
        let content = "- Research: 2024-01-15, 10d\n- Design: 5d, after Research\n- Build: 2024-02-01, 2024-03-01";
        let data = parse_gantt(content);
        assert_eq!(data.tasks.len(), 3);
        assert_eq!(data.tasks[0].name, "Research");
        assert!(data.tasks[0].start.is_some());
        assert!(data.tasks[0].duration.is_some());
        assert_eq!(data.tasks[1].name, "Design");
        assert_eq!(data.tasks[1].dependencies.len(), 1);
        assert_eq!(data.tasks[1].dependencies[0].0, "Research");
        assert_eq!(data.tasks[2].name, "Build");
        assert!(data.tasks[2].start.is_some());
        assert!(data.tasks[2].end.is_some());
    }

    #[test]
    fn test_parse_gantt_with_delay() {
        let content = "- A: 2024-01-01, 5d\n- B: 3d, after A + 2d";
        let data = parse_gantt(content);
        assert_eq!(data.tasks[1].dependencies[0].0, "A");
        assert!(data.tasks[1].dependencies[0].1.is_some());
    }

    #[test]
    fn test_parse_gantt_reveal_markers() {
        let content = "- A: 2024-01-01, 5d\n+ B: 3d, after A\n* C: 2d, after A";
        let data = parse_gantt(content);
        assert_eq!(data.tasks[0].reveal, VizReveal::Static);
        assert_eq!(data.tasks[1].reveal, VizReveal::NextStep);
        assert_eq!(data.tasks[2].reveal, VizReveal::Static);
    }

    #[test]
    fn test_resolve_tasks_basic() {
        let content = "- Research: 2024-01-15, 10d\n- Design: 5d, after Research";
        let data = parse_gantt(content);
        let resolved = resolve_tasks(&data);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].start.format(), "2024-01-15");
        assert_eq!(resolved[0].end.format(), "2024-01-25");
        assert_eq!(resolved[1].start.format(), "2024-01-25");
        assert_eq!(resolved[1].end.format(), "2024-01-30");
    }

    #[test]
    fn test_resolve_tasks_with_delay() {
        let content = "- A: 2024-01-01, 5d\n- B: 3d, after A + 2d";
        let data = parse_gantt(content);
        let resolved = resolve_tasks(&data);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].end.format(), "2024-01-06");
        // B starts 2 days after A ends
        assert_eq!(resolved[1].start.format(), "2024-01-08");
        assert_eq!(resolved[1].end.format(), "2024-01-11");
    }

    #[test]
    fn test_resolve_tasks_parallel() {
        let content = "- Planning: 2024-01-01, 5d\n- Frontend: 10d, after Planning\n- Backend: 10d, after Planning";
        let data = parse_gantt(content);
        let resolved = resolve_tasks(&data);
        assert_eq!(resolved.len(), 3);
        // Frontend and Backend should start on same date
        assert_eq!(resolved[1].start, resolved[2].start);
    }

    #[test]
    fn test_split_specs() {
        let specs = split_specs("5d, after Research");
        assert_eq!(specs, vec!["5d", "after Research"]);

        let specs = split_specs("after A + 3d, 10d");
        assert_eq!(specs, vec!["after A + 3d", "10d"]);
    }

    #[test]
    fn test_workday_duration() {
        let content = "- Task: 2024-01-15, 5wd"; // Monday + 5 workdays
        let data = parse_gantt(content);
        let resolved = resolve_tasks(&data);
        assert_eq!(resolved[0].start.format(), "2024-01-15");
        assert_eq!(resolved[0].end.format(), "2024-01-22"); // Next Monday
    }

    #[test]
    fn test_resolved_tasks_keep_dependencies_when_a_task_is_dropped() {
        // "Ghost" has no dates and no resolvable dependency, so it is dropped.
        // "Build" (index 2 in the source) must still point at "Research"
        // (index 0), not at whatever sits at index 2 of the resolved list.
        let content =
            "- Research: 2024-01-01, 5d\n- Ghost: after Nothing\n- Build: 3d, after Research";
        let data = parse_gantt(content);
        assert_eq!(data.tasks.len(), 3);
        let resolved = resolve_tasks(&data);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[1].name, "Build");
        assert_eq!(resolved[1].dependencies.len(), 1);
        assert_eq!(resolved[1].dependencies[0].0, "Research");
        assert!(resolved[0].dependencies.is_empty());
    }
}
