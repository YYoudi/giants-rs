//! Forward registration traversal with a post-callback successor read.
//! The visitor supplies actual work; no actor callback or world phase is implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Fresh,
    Entry,
    Visit,
    Successor,
    Finished,
    Failed,
}
pub trait Services {
    type Node: Copy;
    fn first(&self) -> Result<Option<Self::Node>, String>;
    fn object(&self, node: Self::Node) -> Result<u64, String>;
    fn next(&self, node: Self::Node) -> Result<Option<Self::Node>, String>;
    fn visit(&mut self, object: u64) -> Result<Flow, String>;
}
#[derive(Debug)]
pub struct Walk {
    phase: Phase,
    visits: usize,
    budget: usize,
    failure: Option<String>,
}
impl Walk {
    /// A finite host budget is explicit; it is not a measured native limit.
    pub fn new(budget: usize) -> Result<Self, String> {
        if budget == 0 {
            return Err("list walk requires a positive visit budget".into());
        }
        Ok(Self {
            phase: Phase::Fresh,
            visits: 0,
            budget,
            failure: None,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn visits(&self) -> usize {
        self.visits
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn run(&mut self, services: &mut impl Services) -> Result<Outcome, String> {
        if self.phase != Phase::Fresh {
            return Err("list walk already consumed or failed".into());
        }
        let result = self.run_inner(services);
        match &result {
            Ok(_) => self.phase = Phase::Finished,
            Err(error) => {
                self.phase = Phase::Failed;
                self.failure = Some(error.clone());
            }
        }
        result
    }
    fn run_inner(&mut self, services: &mut impl Services) -> Result<Outcome, String> {
        self.phase = Phase::Entry;
        let mut current = services.first()?;
        while let Some(node) = current {
            if self.visits >= self.budget {
                return Err("list walk visit budget exhausted".into());
            }
            let object = services.object(node)?;
            self.phase = Phase::Visit;
            self.visits += 1;
            if services.visit(object)? == Flow::Stop {
                return Ok(Outcome::Stopped);
            }
            self.phase = Phase::Successor;
            current = services.next(node)?;
        }
        Ok(Outcome::Completed)
    }
}
