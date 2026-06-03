use std::io::Write;
use wirelog::Logger;

// cargo run --example layered_static_dispatch

fn main() {
    let logger = Logger::new(std::io::stdout());

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

    let repo = Repository::new(logger.clone());
    let service = Service::new(logger.clone(), repo);
    let controller = Controller::new(logger.clone(), service);

    let result = controller.handle_double_data();
    println!("result: {:?}", result);
}

struct Controller<W> {
    logger: Logger<W>,
    service: Service<W>,
}

impl<W: Write + Send + 'static> Controller<W> {
    fn new(logger: Logger<W>, service: Service<W>) -> Self {
        Self {
            logger: logger.with().str("layer", "controller").logger(),
            service,
        }
    }

    pub fn handle_double_data(&self) -> Vec<i32> {
        self.logger.info().msg("handling double data");
        self.service.double_data()
    }
}

struct Service<W> {
    logger: Logger<W>,
    repository: Repository<W>,
}

impl<W: Write + Send + 'static> Service<W> {
    pub fn new(logger: Logger<W>, repository: Repository<W>) -> Self {
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

struct Repository<W> {
    logger: Logger<W>,
    data: Vec<i32>,
}

impl<W: Write + Send + 'static> Repository<W> {
    pub fn new(logger: Logger<W>) -> Self {
        Self {
            logger: logger.with().str("layer", "repository").logger(),
            data: Vec::from([1, 2, 3, 4, 5, 6, 7]),
        }
    }

    pub fn get(&self) -> &[i32] {
        self.logger.info().msg("retrieving data");
        &self.data
    }
}
