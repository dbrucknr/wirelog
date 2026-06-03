use wirelog::{AnyLogger, Logger};

// cargo run --example layered_dyn_dispatch

fn main() {
    let logger = Logger::boxed(std::io::stdout());

    // Clone is cheap here — each clone shares the same Arc<Mutex<W>> writer.
    // The sublogger created in each constructor's .with() call inherits the
    // same sink; no data is duplicated.
    //
    // If you prefer, you can also use .with() calls directly in the constructor.
    // let repo = Repository::new(logger.with().str("layer", "repository").logger());
    // let service = Service::new(logger.with().str("layer", "service").logger(), repo);
    // let controller = Controller::new(logger.with().str("layer", "controller").logger(), service);
    // **NOTE**: if you choose to use the .with() calls directly, you'll need to clean up the constructors
    // in this example.

    let repository = Repository::new(logger.clone(), vec![1, 2, 3]);
    let service = Service::new(logger.clone(), repository);
    let controller = Controller::new(logger.clone(), service);

    let result = controller.handle_double_data();
    println!("result: {:?}", result);
}

struct Controller {
    logger: AnyLogger,
    service: Service,
}

impl Controller {
    fn new(logger: AnyLogger, service: Service) -> Self {
        Self {
            logger: logger.with().str("layer", "controller").logger(),
            service,
        }
    }

    fn handle_double_data(&self) -> Vec<i32> {
        self.logger.info().msg("handling double data");
        self.service.double_data()
    }
}

struct Service {
    logger: AnyLogger,
    repository: Repository,
}

impl Service {
    fn new(logger: AnyLogger, repository: Repository) -> Self {
        Self {
            logger: logger.with().str("layer", "service").logger(),
            repository,
        }
    }

    pub fn double_data(&self) -> Vec<i32> {
        self.logger.info().msg("doubling data");
        self.repository.get().iter().map(|x| x * 2).collect()
    }
}

struct Repository {
    logger: AnyLogger,
    data: Vec<i32>,
}
impl Repository {
    fn new(logger: AnyLogger, data: Vec<i32>) -> Self {
        Self {
            logger: logger.with().str("layer", "repository").logger(),
            data,
        }
    }

    pub fn get(&self) -> &[i32] {
        self.logger.info().msg("retrieving data");
        &self.data
    }
}
