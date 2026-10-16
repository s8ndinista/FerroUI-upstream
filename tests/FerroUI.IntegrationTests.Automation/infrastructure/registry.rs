//! The tests of the run: each with its collection (the application it
//! drives), the platforms it runs on, the reason it is skipped, and how it is
//! run: the constructor of its class, the test, the `Dispose` of its class.

use super::default_app_fixture::{Collection, DefaultAppFixture};
use super::platform_fact::TestPlatforms;
use super::test_base::TestClass;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

/// One test (one row of a theory).
pub struct TestCase {
    pub name: String,
    pub collection: Collection,
    pub platforms: TestPlatforms,
    /// The reason of a `[PlatformFact]` for the platforms it leaves out.
    pub platform_reason: Option<String>,
    /// Why the test does not run: upstream's `Skip`, or a capability the
    /// harness does not provide on the platform of the run.
    pub skip: Option<String>,
    run: Box<dyn Fn(&DefaultAppFixture)>,
}

impl TestCase {
    /// Runs the test: the constructor of the class, the test and the
    /// `Dispose` of the class, which runs however the test ended. A failure
    /// of either fails the test.
    pub fn run(&self, fixture: &DefaultAppFixture) {
        (self.run)(fixture);
    }
}

/// The tests of the run, as the classes register them.
#[derive(Default)]
pub struct Tests {
    pub cases: Vec<TestCase>,
}

/// How a class registers its tests.
pub struct ClassTests<'a, C> {
    tests: &'a mut Tests,
    module: &'static str,
    collection: Collection,
    new: std::rc::Rc<dyn Fn(&DefaultAppFixture) -> C>,
}

impl Tests {
    /// The tests of a class whose tests drive the application of
    /// `collection`, and whose constructor is `new`.
    pub fn class<C: TestClass + 'static>(
        &mut self,
        module: &'static str,
        collection: Collection,
        new: impl Fn(&DefaultAppFixture) -> C + 'static,
    ) -> ClassTests<'_, C> {
        ClassTests { tests: self, module, collection, new: std::rc::Rc::new(new) }
    }
}

impl<C: TestClass + 'static> ClassTests<'_, C> {
    /// `[Fact]`.
    pub fn fact(&mut self, name: &str, test: impl Fn(&C) + 'static) -> &mut Self {
        self.add(name, TestPlatforms::ALL, None, test)
    }

    /// `[PlatformFact(platforms)]`.
    pub fn platform_fact(&mut self, name: &str, platforms: TestPlatforms, test: impl Fn(&C) + 'static) -> &mut Self {
        self.add(name, platforms, None, test)
    }

    /// `[PlatformFact(platforms, reason)]`: the reason is part of the message
    /// of the platforms the test does not run on.
    pub fn platform_fact_because(
        &mut self,
        name: &str,
        platforms: TestPlatforms,
        reason: &str,
        test: impl Fn(&C) + 'static,
    ) -> &mut Self {
        self.add(name, platforms, None, test);
        self.tests.cases.last_mut().expect("the test just added").platform_reason = Some(reason.to_owned());
        self
    }

    /// The test just added needs a capability the harness does not provide on
    /// `platforms`: there it does not run, and the run reports the reason (an
    /// argument `--include-ignored` runs it all the same).
    pub fn not_provided_on(&mut self, platforms: TestPlatforms, reason: &str) -> &mut Self {
        if platforms.is_supported() {
            let case = self.tests.cases.last_mut().expect("the test just added");
            case.skip.get_or_insert_with(|| format!("not provided by the harness: {reason}"));
        }
        self
    }

    /// A test with every option: the platforms and the reason it is skipped.
    pub fn add(
        &mut self,
        name: &str,
        platforms: TestPlatforms,
        skip: Option<&str>,
        test: impl Fn(&C) + 'static,
    ) -> &mut Self {
        let new = self.new.clone();
        self.tests.cases.push(TestCase {
            name: format!("{}::{}", self.module, name),
            collection: self.collection,
            platforms,
            platform_reason: None,
            skip: skip.map(str::to_owned),
            run: Box::new(move |fixture| {
                let instance = new(fixture);
                let result = catch_unwind(AssertUnwindSafe(|| test(&instance)));
                let disposed = catch_unwind(AssertUnwindSafe(|| instance.dispose()));
                if let Err(panic) = result {
                    resume_unwind(panic);
                }
                if let Err(panic) = disposed {
                    resume_unwind(panic);
                }
            }),
        });
        self
    }
}
