use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use itertools::Itertools;

pub fn test_sampler_uniform<T: Eq + Hash + Debug + Copy>(
    expected: &Vec<T>,
    print: bool,
    mut sampler: impl FnMut() -> Option<T>,
) {
    // expected is not a HashSet  so we can print things in a reasonable order
    assert!(
        expected.iter().all_unique(),
        "Got duplicate value in expected: {:?}",
        expected
    );

    // if there are not expected value, ensure the sampler doesn't return any
    if expected.is_empty() {
        for _ in 0..100 {
            assert_eq!(None, sampler());
        }
        return;
    }

    let samples_per_value = 1000;
    let total_samples = samples_per_value * expected.len();

    if print {
        println!(
            "Test sampler uniform: {} values, {} samples/value => {} samples",
            expected.len(),
            samples_per_value,
            total_samples
        );
    }

    // collect samples
    let mut all_counts: HashMap<T, u64> = expected.iter().map(|&value| (value, 0)).collect();

    for _ in 0..total_samples {
        let sample = sampler().expect("There are expected values, so sampler must return one");

        match all_counts.get_mut(&sample) {
            None => panic!("Non-expected value {:?} was sampled", sample),
            Some(count) => *count += 1,
        }
    }

    // print counts
    if print {
        for (&value, &count) in &all_counts {
            println!(
                "  value {:?} sampled {} ~ {}",
                value,
                count,
                count as f32 / samples_per_value as f32
            );
        }
    }

    // check basic correctness
    //   do this separately so worse errors are reported first
    for value in expected {
        assert!(
            *all_counts.get(value).unwrap() > 0,
            "Never sampled expected value {:?}",
            value
        );
    }

    // check uniformity
    for value in expected {
        let count = *all_counts.get(value).unwrap();
        let relative = count as f32 / samples_per_value as f32;

        assert!(
            (0.8..1.2).contains(&relative),
            "Value {:?} was over/under sampled {} ~ {}",
            value,
            count,
            relative,
        );
    }
}
