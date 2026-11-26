//! Screenshot evidence and one checked approval tied to a fresh native run.
use crate::{scenario::read_text, scenario_images};
use ferroui_designer_project::{scenario_baselines as files, ProjectError};
use ferroui_designer_tools::scenario::{
    BaselineReference, ComparisonStatus, ScenarioReport, ScenarioStepReport, VisualReview,
};
use ferroui_reload_protocol::fingerprint;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) enum Action {
    Verify,
    Accept,
    Review,
    Approve(String),
}

impl Action {
    fn reviews(&self) -> bool {
        matches!(self, Self::Review | Self::Approve(_))
    }
}

pub(super) struct Options {
    pub(super) action: Action,
    pub(super) reference: Option<String>,
}

pub(super) struct Inputs<'a> {
    pub(super) scenario: &'a Path,
    pub(super) scenario_text: &'a str,
    pub(super) document: &'a Path,
    pub(super) document_hash: u64,
    pub(super) workspace: &'a Path,
    pub(super) artifacts: &'a Path,
}

pub(super) struct Review<'a> {
    input: Inputs<'a>,
    options: &'a Options,
    repository: Option<files::Repository>,
    paths: BTreeSet<PathBuf>,
    candidates: Vec<Candidate>,
    pub(super) visuals: Vec<VisualReview>,
}

struct Candidate {
    step: usize,
    baseline: PathBuf,
    expected: Option<Vec<u8>>,
    image: Vec<u8>,
}

impl<'a> Review<'a> {
    pub(super) fn new(input: Inputs<'a>, options: &'a Options) -> Result<Self, String> {
        let repository = if options.action.reviews() {
            files::Repository::open(
                input.scenario.parent().expect("scenario directory"),
                options.reference.as_deref(),
            )
            .map_err(error)?
        } else {
            None
        };
        Ok(Self {
            input,
            options,
            repository,
            paths: BTreeSet::new(),
            candidates: Vec::new(),
            visuals: Vec::new(),
        })
    }

    pub(super) fn reference(&self) -> Option<BaselineReference> {
        Some(BaselineReference {
            name: self.options.reference.clone()?,
            commit: self.repository.as_ref()?.ancestor()?.into(),
        })
    }

    pub(super) fn continues_after_difference(&self) -> bool {
        self.options.action.reviews()
    }

    pub(super) fn capture(
        &mut self,
        name: &str,
        step: usize,
        image: Vec<u8>,
        tolerance: u8,
        maximum: u32,
    ) -> Result<ScenarioStepReport, String> {
        let base = self.input.scenario.parent().expect("scenario directory");
        let baseline = files::destination(base, self.input.workspace, name).map_err(error)?;
        if !matches!(self.options.action, Action::Verify) && !self.paths.insert(baseline.clone()) {
            return Err("Give each captured screenshot its own baseline file".into());
        }
        let expected = files::read(&baseline).map_err(error)?;
        let candidate = Candidate {
            step,
            baseline,
            expected,
            image,
        };
        if matches!(self.options.action, Action::Accept) {
            let artifacts = vec![candidate.baseline.clone()];
            self.candidates.push(candidate);
            return Ok(ScenarioStepReport {
                step,
                passed: true,
                message: "Captured screenshot for acceptance".into(),
                artifacts,
            });
        }
        let local = scenario_images::compare(
            &candidate.image,
            candidate.expected.as_deref(),
            tolerance,
            maximum,
        )?;
        let passed = local.summary.status == ComparisonStatus::Match;
        let message = local.message(&candidate.baseline);
        let artifacts = if self.options.action.reviews() {
            self.compare(&candidate, &local, tolerance, maximum)?
        } else if passed {
            Vec::new()
        } else {
            let mut result = Vec::new();
            if let Some(bytes) = candidate.expected.as_deref() {
                result.push(scenario_images::record(
                    self.input.artifacts,
                    step,
                    "expected",
                    bytes,
                )?);
            }
            result.push(scenario_images::record(
                self.input.artifacts,
                step,
                "actual",
                &candidate.image,
            )?);
            if let Some(bytes) = local.difference.as_deref() {
                result.push(scenario_images::record(
                    self.input.artifacts,
                    step,
                    "diff",
                    bytes,
                )?);
            }
            result
        };
        if let Action::Approve(request) = &self.options.action {
            if self.visuals.last().and_then(|item| item.approval.as_ref()) == Some(request) {
                self.candidates.push(candidate);
            }
        }
        Ok(ScenarioStepReport {
            step,
            passed,
            message,
            artifacts,
        })
    }

    fn compare(
        &mut self,
        candidate: &Candidate,
        local: &scenario_images::ComparedImage,
        tolerance: u8,
        maximum: u32,
    ) -> Result<Vec<PathBuf>, String> {
        let repository = self
            .repository
            .as_ref()
            .filter(|repo| repo.ancestor().is_some());
        let history = repository
            .map(|repo| repo.baseline(&candidate.baseline))
            .transpose()
            .map_err(error)?;
        let displayed = history
            .as_ref()
            .map(|value| value.as_deref())
            .unwrap_or(candidate.expected.as_deref());
        let comparison = if repository.is_some() {
            Some(scenario_images::compare(
                &candidate.image,
                displayed,
                tolerance,
                maximum,
            )?)
        } else {
            None
        };
        let comparison = comparison.as_ref().unwrap_or(local);
        let step = candidate.step;
        let write = |kind, bytes| scenario_images::record(self.input.artifacts, step, kind, bytes);
        let actual = write("actual", &candidate.image)?;
        let expected = displayed
            .map(|value| write("expected", value))
            .transpose()?;
        let difference = comparison
            .difference
            .as_deref()
            .map(|value| write("diff", value))
            .transpose()?;
        let approval = (local.summary.status != ComparisonStatus::Match)
            .then(|| self.receipt(candidate))
            .transpose()?;
        let artifacts = expected
            .iter()
            .chain([&actual])
            .chain(difference.iter())
            .cloned()
            .collect();
        self.visuals.push(VisualReview {
            step,
            baseline: candidate.baseline.clone(),
            workspace: local.summary,
            comparison: comparison.summary,
            actual,
            expected,
            difference,
            approval,
        });
        Ok(artifacts)
    }

    fn receipt(&self, candidate: &Candidate) -> Result<String, String> {
        let identity = (
            self.input.workspace,
            self.input.scenario,
            fingerprint(self.input.scenario_text.as_bytes()),
            self.input.document,
            self.input.document_hash,
            &self.repository,
            &self.options.reference,
            candidate.step,
            &candidate.baseline,
            candidate.expected.as_deref().map(fingerprint),
            fingerprint(&candidate.image),
        );
        let bytes = serde_json::to_vec(&identity).map_err(|error| error.to_string())?;
        Ok(format!("{:016x}", fingerprint(&bytes)))
    }

    pub(super) fn finish(
        mut self,
        report: &mut ScenarioReport,
        assertions_passed: bool,
    ) -> Result<(), String> {
        if !assertions_passed {
            for review in &mut self.visuals {
                review.approval = None;
            }
        }
        report.visuals = std::mem::take(&mut self.visuals);
        if matches!(self.options.action, Action::Approve(_)) {
            if !assertions_passed {
                return Err("Fix the scenario assertions before approving a baseline".into());
            }
            if self.candidates.len() != 1 {
                return Err("The reviewed capture, source, or accepted baseline changed. Run the review again.".into());
            }
        }
        if !assertions_passed {
            return Ok(());
        }
        for candidate in &self.candidates {
            files::publish(
                self.input.workspace,
                &candidate.baseline,
                candidate.expected.as_deref(),
                &candidate.image,
                || self.validate_source(),
            )
            .map_err(error)?;
            let step = &mut report.steps[candidate.step - 1];
            step.passed = true;
            step.message = format!("Accepted {}", candidate.baseline.display());
            if matches!(self.options.action, Action::Approve(_)) {
                report.approved = Some(candidate.baseline.clone());
                if let Some(review) = report
                    .visuals
                    .iter_mut()
                    .find(|item| item.step == candidate.step)
                {
                    review.approval = None;
                    review.workspace.status = ComparisonStatus::Match;
                    review.workspace.changed_pixels = Some(0);
                    review.workspace.expected = Some(review.workspace.actual);
                }
            }
        }
        report.passed = report.steps.iter().all(|step| step.passed);
        Ok(())
    }

    fn validate_source(&self) -> ferroui_designer_project::Result<()> {
        let scenario = read_text(self.input.scenario).map_err(ProjectError)?;
        let document = read_text(self.input.document).map_err(ProjectError)?;
        let repository_changed = self.options.action.reviews()
            && files::Repository::open(
                self.input.scenario.parent().expect("scenario directory"),
                self.options.reference.as_deref(),
            )? != self.repository;
        if scenario != self.input.scenario_text
            || fingerprint(document.as_bytes()) != self.input.document_hash
            || repository_changed
        {
            return Err(ProjectError(
                "The scenario or source changed. Run the review again.".into(),
            ));
        }
        Ok(())
    }
}

fn error(error: ProjectError) -> String {
    error.to_string()
}
