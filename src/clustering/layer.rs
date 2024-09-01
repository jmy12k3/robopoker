use crate::cards::observation::Observation;
use crate::cards::street::Street;
use crate::clustering::abstraction::Abstraction;
use crate::clustering::consumer::Consumer;
use crate::clustering::histogram::Histogram;
use crate::clustering::metric::Metric;
use crate::clustering::producer::Producer;
use crate::clustering::progress::Progress;
use crate::clustering::projection::Projection;
use crate::clustering::xor::Pair;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::Type;
use tokio_postgres::Client;

/// KMeans hiearchical clustering. Every Observation is to be clustered with "similar" observations. River cards are the base case, where similarity metric is defined by equity. For each higher layer, we compare distributions of next-layer outcomes. Distances are measured by EMD and unsupervised kmeans clustering is used to cluster similar distributions. Potential-aware imperfect recall!
pub struct Layer {
    street: Street,
    metric: BTreeMap<Pair, f32>, // impl Metric
    points: BTreeMap<Observation, (Histogram, Abstraction)>, // impl Projection
    kmeans: BTreeMap<Abstraction, (Histogram, Histogram)>,
}

impl Layer {
    /// Upload to database. We'll open a new connection for each layer, whatever.
    pub async fn upload(self) -> Self {
        println!("uploading {}", self.street);
        let ref url = std::env::var("DATABASE_URL").expect("DATABASE_URL in environment");
        let (ref client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls)
            .await
            .expect("connect to database");
        tokio::spawn(connection);
        self.truncate(client).await;
        self.upload_distance(client).await;
        self.upload_centroid(client).await;
        self
    }

    /// async equity calculations to create initial River layer.
    pub async fn outer() -> Self {
        Self {
            street: Street::Rive,
            kmeans: BTreeMap::default(),
            metric: Self::outer_metric(),
            points: Self::outer_points().await,
        }
    }

    /// Yield the next layer of abstraction by kmeans clustering. The recursive nature of layer methods encapsulates the hiearchy of learned abstractions via kmeans.
    /// TODO; make this async and persist to database after each layer
    pub fn inner(&self) -> Self {
        let mut inner = Self {
            street: self.street.prev(),
            kmeans: self.inner_kmeans(),
            metric: self.inner_metric(),
            points: self.inner_points(),
        };
        inner.cluster();
        inner
    }

    /// Number of centroids in k means on inner layer. Loosely speaking, the size of our abstraction space.
    fn k(&self) -> usize {
        match self.street.prev() {
            Street::Turn => 500,
            Street::Flop => 500,
            Street::Pref => 169,
            _ => unreachable!("no other prev"),
        }
    }

    /// Number of kmeans iterations to run on current layer.
    fn t(&self) -> usize {
        match self.street.prev() {
            Street::Turn => 100,
            Street::Flop => 100,
            Street::Pref => 10,
            _ => unreachable!("no other prev"),
        }
    }

    /// Run kmeans iterations.
    /// Presumably, we have been generated by a previous layer, with the exception of Outer == River.
    /// After the base case, we trust that our observations, abstractions, and metric are correctly populated.
    fn cluster(&mut self) {
        println!("clustering kmeans {} < {}", self.street.prev(), self.street);
        let t = self.t();
        let ref mut progress = Progress::new(t, 10);
        for _ in 0..t {
            // find nearest neighbor. shift centroid accordingly
            for (_, (data, last)) in self.points.iter_mut() {
                let mut nearests = f32::MAX;
                let mut neighbor = Abstraction::default();
                for (centroid, (mean, _)) in self.kmeans.iter_mut() {
                    let distance = self.metric.emd(data, mean);
                    if distance < nearests {
                        nearests = distance;
                        neighbor = *centroid;
                    }
                }
                // update nearest neighbor abstraction of this observation
                let ref mut neighbor = neighbor;
                self.kmeans
                    .get_mut(neighbor)
                    .expect("replaced default abstraction")
                    .0
                    .absorb(data);
                std::mem::swap(last, neighbor);
            }
            // swap old and new centroids. prepare for next iteration
            for (_, (old, new)) in self.kmeans.iter_mut() {
                old.clear();
                std::mem::swap(old, new);
            }
            progress.tick();
        }
    }

    /// Compute the metric of the next innermost layer. Take outer product of centroid histograms over measure.
    fn inner_metric(&self) -> BTreeMap<Pair, f32> {
        println!("computing metric {} < {}", self.street.prev(), self.street);
        let ref centroids = self.kmeans;
        let mut metric = BTreeMap::new();
        for (i, (x, _)) in centroids.iter().enumerate() {
            for (j, (y, _)) in centroids.iter().enumerate() {
                if i > j {
                    let index = Pair::from((x, y));
                    let ref x = centroids.get(x).expect("in centroids").0;
                    let ref y = centroids.get(y).expect("in centroids").0;
                    let distance = self.metric.emd(x, y);
                    metric.insert(index, distance);
                }
            }
        }
        metric
    }

    /// Generate all possible obersvations of the next innermost layer.
    /// Assign them to arbitrary abstractions. They will be overwritten during kmeans iterations.
    /// Base case is River which comes from equity bucket calculation.
    fn inner_points(&self) -> BTreeMap<Observation, (Histogram, Abstraction)> {
        println!("projecting {} < {}", self.street.prev(), self.street);
        Observation::all(self.street.prev())
            .into_iter()
            .map(|inner| (inner, (self.points.project(inner), Abstraction::default())))
            .collect()
    }

    /// K Means++ implementation yields initial histograms
    /// Abstraction labels are random and require uniqueness.
    fn inner_kmeans(&self) -> BTreeMap<Abstraction, (Histogram, Histogram)> {
        println!("choosing means {} < {}", self.street.prev(), self.street);
        use rand::distributions::Distribution;
        use rand::distributions::WeightedIndex;
        use rand::seq::SliceRandom;
        // 0. Initialize data structures
        let mut kmeans = Vec::new();
        let ref mut histograms = self.points.values().map(|(histogram, _)| histogram);
        let ref mut rng = rand::thread_rng();
        // 1. Choose 1st centroid randomly from the dataset
        let sample = histograms
            .collect::<Vec<&Histogram>>()
            .choose(rng)
            .expect("non-empty lower observations")
            .to_owned()
            .clone();
        kmeans.push(sample);
        // 2. Choose nth centroid with probability proportional to squared distance of nearest neighbors
        while kmeans.len() < self.k() {
            let distances = histograms
                .map(|histogram| {
                    kmeans
                        .iter()
                        .map(|initial| self.metric.emd(initial, histogram))
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .expect("find minimum")
                })
                .map(|min| min * min)
                .collect::<Vec<f32>>();
            let choice = WeightedIndex::new(distances)
                .expect("valid weights array")
                .sample(rng);
            let sample = histograms
                .nth(choice)
                .expect("shared index with lowers")
                .to_owned();
            kmeans.push(sample);
        }
        // 3. Collect histograms and label with arbitrary (random) Abstractions
        kmeans
            .into_iter()
            .map(|mean| (Abstraction::random(), (mean, Histogram::default())))
            .collect::<BTreeMap<_, _>>()
    }

    /// Generate the  baseline metric between equity bucket abstractions. Keeping the u64->f32 conversion is fine for distance since it preserves distance
    fn outer_metric() -> BTreeMap<Pair, f32> {
        println!("calculating equity bucket metric");
        let mut metric = BTreeMap::new();
        for i in 0..Abstraction::EQUITIES as u64 {
            for j in i..Abstraction::EQUITIES as u64 {
                let distance = (j - i) as f32;
                let ref i = Abstraction::from(i);
                let ref j = Abstraction::from(j);
                let index = Pair::from((i, j));
                metric.insert(index, distance);
            }
        }
        metric
    }

    // construct observation -> abstraction map via equity calculations
    async fn outer_points() -> BTreeMap<Observation, (Histogram, Abstraction)> {
        println!("calculating equity bucket observations");
        let ref observations = Arc::new(Observation::all(Street::Rive));
        let (tx, rx) = tokio::sync::mpsc::channel::<(Observation, Abstraction)>(1024);
        let consumer = Consumer::new(rx);
        let consumer = tokio::spawn(consumer.run());
        let producers = (0..num_cpus::get())
            .map(|i| Producer::new(i, tx.clone(), observations.clone()))
            .map(|p| tokio::spawn(p.run()))
            .collect::<Vec<_>>();
        std::mem::drop(tx);
        futures::future::join_all(producers).await;
        consumer.await.expect("equity mapping task completes")
    }
}

/// SQL operations
impl Layer {
    /// Truncate the database tables
    async fn truncate(&self, client: &Client) {
        if self.street == Street::Rive {
            client
                .batch_execute(
                    r#"
                    DROP TABLE IF EXISTS centroid;
                    DROP TABLE IF EXISTS distance;
                    CREATE UNLOGGED TABLE centroid (
                        observation BIGINT PRIMARY KEY,
                        abstraction BIGINT
                    );
                    CREATE UNLOGGED TABLE distance (
                        xor         BIGINT PRIMARY KEY,
                        distance    REAL
                    );
                    "#,
                )
                .await
                .expect("nuke");
        }
    }

    /// Upload centroid data to the database
    /// would love to be able to FREEZE table for initial river COPY
    async fn upload_centroid(&self, client: &Client) {
        let sink = client
            .copy_in(
                r#"
                COPY centroid (
                    observation,
                    abstraction
                )
                FROM STDIN BINARY;
                "#,
            )
            .await
            .expect("get sink for COPY transaction");
        let ref mut writer = BinaryCopyInWriter::new(sink, &[Type::INT8, Type::INT8]);
        let mut writer = unsafe { Pin::new_unchecked(writer) };
        let mut progress = Progress::new(self.points.len(), 10_000_000);
        for (observation, (_, abstraction)) in self.points.iter() {
            writer
                .as_mut()
                .write(&[observation, abstraction])
                .await
                .expect("write row into heap");
            progress.tick();
        }
        writer
            .finish()
            .await
            .expect("complete centroid COPY transaction");
    }

    /// Upload distance data to the database
    /// would love to be able to FREEZE table for initial river COPY
    async fn upload_distance(&self, client: &Client) {
        let sink = client
            .copy_in(
                r#"
                COPY distance (
                    xor,
                    distance
                )
                FROM STDIN BINARY;
                "#,
            )
            .await
            .expect("get sink for COPY transaction");
        let ref mut writer = BinaryCopyInWriter::new(sink, &[Type::INT8, Type::FLOAT4]);
        let mut writer = unsafe { Pin::new_unchecked(writer) };
        let mut progress = Progress::new(self.metric.len(), 1_000);
        for (pair, distance) in self.metric.iter() {
            writer
                .as_mut()
                .write(&[pair, distance])
                .await
                .expect("write row into heap");
            progress.tick();
        }
        writer
            .finish()
            .await
            .expect("complete distance COPY transaction");
    }
}
