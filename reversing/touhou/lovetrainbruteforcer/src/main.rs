use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

struct Character {
    pub name: String,
    pub relationships: Vec<(usize, u32)>, // index, score
}

impl Character {
    pub fn new(name: &str) -> Character {
        Character {
            name: name.to_string(),
            relationships: Vec::new(),
        }
    }
}

struct CharacterDataset {
    pub characters: Vec<Character>,
}

impl CharacterDataset {
    pub fn new(chars: &str, pairs: &str) -> CharacterDataset {
        // Load all existing characters
        // pairs.csv has some PC-98 characters, which don't exist in this game
        let mut existing = HashSet::new();
        existing.extend(chars.split(','));

        // Keep track of the name -> index pairing for fast lookup
        let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
        let mut cur_idx = 0;

        // Iterate over every pair in the dataset
        let mut characters = Vec::<Character>::new();
        for line in pairs.split('\n') {
            // Parse the line
            let mut split = line.split(',');
            let left = split.next().unwrap();
            let right = split.next().unwrap();
            let post_count = str::parse::<u32>(split.next().unwrap().trim_end()).unwrap();

            // Skip if a target character doesn't exist
            if !existing.contains(left) || !existing.contains(right) {
                continue;
            }

            // Insert any unknown names into the name -> index map
            if !name_to_idx.contains_key(left) {
                characters.push(Character::new(left));
                name_to_idx.insert(left, cur_idx);
                cur_idx += 1;
            }
            if !name_to_idx.contains_key(right) {
                characters.push(Character::new(right));
                name_to_idx.insert(right, cur_idx);
                cur_idx += 1;
            }

            // Add the pairing to each character's list
            if post_count != 0 {
                let mut score = post_count;

                // Bonus: 10000x for 1 post count
                if post_count == 1 {
                    score *= 10000;
                }

                let left_idx = name_to_idx[left];
                let right_idx = name_to_idx[right];
                characters[left_idx].relationships.push((right_idx, score));
                characters[right_idx].relationships.push((left_idx, score));
            }
        }

        // Do a second pass for bonuses now that all relationships have been added
        for i in 0..characters.len() {
            // Bonus: 10x for characters with only one partner
            if characters[i].relationships.len() == 1 {
                characters[i].relationships[0].1 *= 10;
            }

            // Bonus: 10x for characters that have less than 1000 posts in total (bidirectional)
            if characters[i].relationships.iter().map(|x| x.1).sum::<u32>() < 1000 {
                for j in 0..characters[i].relationships.len() {
                    characters[i].relationships[j].1 *= 10;

                    // Also multiply for pairs in the other direction
                    let other_idx = characters[i].relationships[j].0;
                    characters[other_idx]
                        .relationships
                        .iter_mut()
                        .find(|x| x.0 == i)
                        .unwrap()
                        .1 *= 10;
                }
            }

            // Bonus: 5x for characters without an official name (bidirectional)
            const UNOFFICIAL_NAMES: [&str; 3] = ["大妖精", "小悪魔", "朱鷺子"];
            if UNOFFICIAL_NAMES
                .iter()
                .find(|x| characters[i].name == **x)
                .is_some()
            {
                for j in 0..characters[i].relationships.len() {
                    characters[i].relationships[j].1 *= 5;

                    // Also multiply for pairs in the other direction
                    let other_idx = characters[i].relationships[j].0;
                    characters[other_idx]
                        .relationships
                        .iter_mut()
                        .find(|x| x.0 == i)
                        .unwrap()
                        .1 *= 5;
                }
            }

            // Bonus: 5x for everyone introduced after WBaWC (bidirectional)
            const NEWCOMERS: [&str; 9] = [
                "山城たかね",
                "豪徳寺ミケ",
                "姫虫百々世",
                "飯綱丸龍",
                "駒草山如",
                "玉造魅須丸",
                "饕餮尤魔",
                "菅牧典",
                "天弓千亦",
            ];
            if NEWCOMERS
                .iter()
                .find(|x| characters[i].name == **x)
                .is_some()
            {
                for j in 0..characters[i].relationships.len() {
                    characters[i].relationships[j].1 *= 5;

                    // Also multiply for pairs in the other direction
                    let other_idx = characters[i].relationships[j].0;
                    characters[other_idx]
                        .relationships
                        .iter_mut()
                        .find(|x| x.0 == i)
                        .unwrap()
                        .1 *= 5;
                }
            }
        }

        CharacterDataset { characters }
    }
}

struct TrainWalker<'a> {
    pub train: Vec<(usize, u32)>, // index, reward from pair with previous character
    pub train_score: u32,
    dataset: &'a CharacterDataset,
    visited: Vec<bool>,
    rand: ThreadRng,

    choices: Vec<(usize, u32)>,
}

impl TrainWalker<'_> {
    pub fn new<'a>(dataset: &'a CharacterDataset, initial: &str) -> TrainWalker<'a> {
        // Lookup doesn't have to be fast since this only happens at the start of execution
        let inital_idx = dataset
            .characters
            .iter()
            .enumerate()
            .find(|x| x.1.name == initial)
            .expect("character not found")
            .0;

        TrainWalker {
            train: vec![(inital_idx, 0)],
            train_score: 0,
            dataset,
            visited: vec![false; dataset.characters.len()],
            rand: rand::thread_rng(),

            choices: Vec::new(),
        }
    }

    // Randomly walk a path until a dead end is hit
    pub fn walk(&mut self) {
        let mut last = self
            .train
            .last()
            .expect("can't walk if train is empty")
            .clone();

        loop {
            // Reuse vectors to avoid reallocation
            self.choices.clear();

            // Enumerate possible choices
            let relationships = &self.dataset.characters[last.0].relationships;
            self.choices.reserve(relationships.len());
            self.choices
                .extend(relationships.iter().filter(|x| !self.visited[x.0]));

            // Bail if there aren't any more choices
            if self.choices.is_empty() {
                break;
            }

            // Pick a random choice weighted by post count
            let choice = self
                .choices
                .choose_weighted(&mut self.rand, |x| x.1)
                .unwrap();

            let score = {
                // Bonus: 10x multiplier for every 10 dolls connected
                if self.train.len() % 10 == 9 {
                    choice.1 * 10
                } else {
                    choice.1
                }
            };

            // Add the choice to the train
            self.train.push((choice.0, score));
            self.train_score += score;
            self.visited[choice.0] = true;

            // Set it as the last one
            last = (choice.0, score);
        }
    }

    // Reusing objects to not constantly reallocate stuff
    pub fn reset(&mut self, keep: usize) {
        assert!(keep != 0, "tried to clear entire train");
        assert!(
            keep <= self.train.len(),
            "tried to keep more elements than train length"
        );

        // Truncate the train to the amount that's being kept
        self.train.truncate(keep);

        // Reset score and visited characters
        self.train_score = 0;
        self.visited.fill(false);

        // Recalculate them
        for x in &self.train {
            self.train_score += x.1;
            self.visited[x.0] = true;
        }
    }
}

fn print_train(walker: &TrainWalker, speed: f64) {
    print!("\x1b[0;32mNew result: \x1b[0m");

    for (i, x) in walker.train.iter().enumerate() {
        print!("{}", walker.dataset.characters[x.0].name);
        if i != walker.train.len() - 1 {
            print!(" -> ");
        }
    }
    println!();

    println!(
        "{} characters, {} points, {:.2} walks/sec",
        walker.train.len(),
        walker.train_score,
        speed
    );
}

fn main() {
    // Load the dataset
    let dataset = CharacterDataset::new(include_str!("characters.txt"), include_str!("pairs.txt"));
    let mut walker = TrainWalker::new(&dataset, "河城にとり");

    // Search
    let mut rand = rand::thread_rng();
    let mut best_train_score = 0;
    let mut rate_counter: usize = 0;
    let mut last_report = Instant::now();
    loop {
        // Clear a random amount of the train
        walker.reset(rand.gen_range(1..=walker.train.len()));

        // Walk
        walker.walk();

        // Check if this result is better than the last one
        if walker.train_score > best_train_score {
            best_train_score = walker.train_score;

            print_train(
                &walker,
                rate_counter as f64 / last_report.elapsed().as_secs_f64(),
            );
            rate_counter = 0;
            last_report = Instant::now();
        }

        rate_counter += 1;
    }
}
