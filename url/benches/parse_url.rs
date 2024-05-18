#[macro_use]
extern crate bencher;

use bencher::{black_box, Bencher};

use url::Url;

fn short(bench: &mut Bencher) {
    let url = "https://example.com/bench";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn long(bench: &mut Bencher) {
    let url = "https://example.com/parkbench?tre=es&st=uff";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn very_long(bench: &mut Bencher) {
    let url = "https://www.typescriptlang.org/play/?&q=42#code/PTAEBUE8AcFMGUDGAnAltALqVBnUBDUeDZAV0Q1OXwBsIZYjIcNYBbAOlAEFQWyKVWqAwMAUCD7NWbUG1j4AdngwALfFgDuq2ItCIA9m2j40igOYiGOADT04SNJlAHFNSBLAZ8Aa1h5URQwDAkRDUiCRHTl2ACNYZDxXKMZROA4xTwhVXGwAvUMg6hYREMUjQOE0xhxpdltQbQTGSANSLPCaABN9ZAVWEU0Q6rxY0ixOnvKsfBwcVHM9NXZS0AVEVSyDZeQueFhGWAAPfGMaWAAucrZKmgBaNMDzTMkAMQNkNZOz2DtlnFSQ2wQQSADN8Ih-AQ+vojNBzqx3FkSEocKCEtRYudgQQ+CRyJRqHRqlIWOwLplAqxkODIaAAEK0OgAbzEoFAXVQp1g1IuoEUpDY8WQAG4xABfMRUsEQxjwaA6GGs9mc7m8-mC4ViyViBGgWJMvmMmh0AC8oGZHK58nVAEYAAygcVivU4BXNPnyxWMc2W1U2hJ8gBMjudmTd3tA5oNJrFMbNfHdfTFWQAkqDGox8F0eoFcSTtKgNniBISmZBYUF8IE8EyXKDkdF5EKEkkM8a6Eoel7mn8dHpUDMaDgQoglPrYFkAVhghPcfGXJ8I80MlKQTTZRBSPELWyrWrAxqW6K9+cLGo+QLj9rMnrKPE+eBtz6LfuA8g+bag3Yz+YL6AAGYnRTe8X3jOM63NUCU0kelYDHUgAXnOsugMKFplAdQADcs1AX81D7XRsAmJQMPiLJZnmRZYB6WdlhEZ9QCw0wuSxWAuAACQMTRYBw5A7DrAwM3o5thRwLZ2zrUxGBrVAulSZ8BMUHoR2I-Rxx3SiFkUGjV0kKAHBQdAsFyQxjBYiw1ghVQYmPHF6JJfBzGrZQMCydYbO2RVVj41BQQrZZUE+WAAEdSFoQdID0sBeFBCIKFQZJcnHAxYgAK3gky9AAKXwZjHGMghlLU0y4WkrosjzQh5huGhTFAcEcByVwuAAdUHTydNARDYDiuhjhREQ0EQHxoTaZTG0YExqDYHAKV1Hlen6WAOyjUAAAp-R5Q8r2FABKKMAD4Nr9a1ts+cU9pdRaUGWnsYXNTazvVXaEjsHrU0UDZ-D5WIDAMc4lAO01juVUA+kJPRToPD9uoBT7vrwAB+N9ztAAAqUB7Q4ACAE5QD5LbqWAiUU1ujQECTF9ydYDsxRplbIKWin7tgGCwAMhAjOcTRUBNAgTW4jbXuQA7Z1C8K6HWkW7D+gGFEUPbKqWaJpphhoxiwDDpc1N79X+wHFdAO5jp148layTmCp5vm6E5HAx2QWjojlw37OiUEgpKLTFnkIIsniBCkMHAByPA+IrMy2GSUEPlAXL8u5iYDHk1YcB8dBQBMOYniyNXZsafsUkgEOYQwnSaN0l4wHeYLvnhVJolMagS7wWPkAAUWssP1JNA0Rsw2Yoj6KbTFOWwsmYmhSF+YF5KOIrncYPrO2QFvjewDMraTq0ukUEO3MkHBSGgaAPiwe3Hc5Sz8-Ohplj0VpSEaNpukHnDSmVxBp9T3iEgrAwmBEpLBCGwXwjdl7xQwMAvAYCMDFhPvNAA2gzDs61bR7TsKgpk60gx7QALocHbl3DY611rxjsAAfUCPPKh8YcDA1BnuQoI5zgcBoAYcw5CmRXQlLwrIAA5EIrhGAVy6CodQGBoqgAAEo8ioEsaw0JUh9Apj0GgqA-AuHSplBoXYioVk5KCdEfQvr+AojCMy00aKNA6ikPg3JtEZQoGsMKEVRDg1IOcWsf0cKrj1AzORXRVqPSJjtXWosjonVRtSLBAMPh8gAER9C6Ikp0-DsH83NIEmidMxA5OCUzTJNB2agFapGeintEgzBzr7XQWgPg+DwOtZYFY-pqHfowMJosA7jHsQCQoPRUL+H3lgFpLsUJoRwKMzpuJDAcNFhkIAA";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

benchmark_group!(benches, short, long, very_long);
benchmark_main!(benches);
