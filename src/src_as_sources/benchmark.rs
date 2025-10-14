use crate::src_as_sources::import_parsing::parse_purescript_file;
use std::time::Instant;

/// Benchmark the parser performance
pub fn benchmark_parser() {
    // Simple case
    let simple_case = "module Simple where\nimport Prelude";

    // Complex case with multiline imports
    let complex_case = r#"module Complex where

import Prelude
import Data.Maybe (Maybe(..), maybe) as Maybe
import Data.Either hiding (left, right)
import Data.List
  ( List(..)
  , (:)
  , head
  , tail
  ) as List
import Data.Tuple (Tuple(..), fst, snd)
import Data.Function (($), (#), (<<<), (>>>))
import Control.Monad (class Monad, bind, pure)
import Effect (Effect)
import Effect.Console (log)

main :: Effect Unit
main = pure unit"#;

    // Very long import list
    let mut long_imports = String::from("module Long where\n");
    for i in 0..1000 {
        long_imports.push_str(&format!("import Data.Module{} (item{})\n", i, i));
    }
    long_imports.push_str("main = pure unit");

    let test_cases = vec![
        simple_case,
        complex_case,
        &long_imports
    ];

    for (i, test_case) in test_cases.iter().enumerate() {
        println!("Benchmarking test case {}...", i + 1);

        let start = Instant::now();
        let iterations = 1000;

        for _ in 0..iterations {
            let _ = parse_purescript_file(test_case);
        }

        let duration = start.elapsed();
        let avg_time = duration.as_nanos() / iterations as u128;

        println!("  Average time per parse: {} ns", avg_time);
        println!("  Total time for {} iterations: {:?}", iterations, duration);
        println!();
    }
}
