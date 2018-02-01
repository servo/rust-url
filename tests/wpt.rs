extern crate data_url;
extern crate rustc_test;
extern crate serde_json;

fn run_data_url(input: String, expected_mime: Option<String>, expected_body: Option<Vec<u8>>) {
    let url = data_url::DataUrl::process(&input);
    if let Some(expected_mime) = expected_mime {
        let url = url.unwrap();
        let (body, _) = url.decode_to_vec().unwrap();
        if expected_mime == "" {
            assert_eq!(*url.mime_type(), "text/plain;charset=US-ASCII")
        } else {
            assert_eq!(*url.mime_type(), &*expected_mime)
        }
        if let Some(expected_body) = expected_body {
            assert_eq!(body, expected_body)
        }
    } else if let Ok(url) = url {
        assert!(url.decode_to_vec().is_err(), "{:?}", url.mime_type())
    }
}

fn collect_data_url<F>(add_test: &mut F)
    where F: FnMut(String, bool, rustc_test::TestFn)
{
    let json = include_str!("data-urls.json");
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    for test in v.as_array().unwrap() {
        let input = test.get(0).unwrap().as_str().unwrap().to_owned();

        let expected_mime = test.get(1).unwrap();
        let expected_mime = if expected_mime.is_null() {
            None
        } else {
            Some(expected_mime.as_str().unwrap().to_owned())
        };

        let expected_body = test.get(2).map(|j| {
            j.as_array().unwrap().iter().map(|byte| {
                let byte = byte.as_u64().unwrap();
                assert!(byte <= 0xFF);
                byte as u8
            }).collect::<Vec<u8>>()
        });

        let should_panic = [
            "data://test:test/,X",
            "data:;%62ase64,WA",
            "data:;base 64,WA",
            "data:;base64;,WA",
            "data:;base64;base64,WA",
            "data:;charset =x,X",
            "data:;charset,X",
            "data:;charset=,X",
            "data:text/plain;,X",
            "data:text/plain;a=\",\",X",
            "data:x/x;base64;base64,WA",
            "data:x/x;base64;base64x,WA",
            "data:x/x;base64;charset=x,WA",
            "data:x/x;base64;charset=x;base64,WA",
        ].contains(&&*input);
        add_test(
            format!("data: URL {:?}", input),
            should_panic,
            rustc_test::TestFn::dyn_test_fn(move || {
                run_data_url(input, expected_mime, expected_body)
            })
        );
    }
}

fn main() {
    let mut tests = Vec::new();
    {
        let mut add_one = |name: String, should_panic: bool, run: rustc_test::TestFn| {
            let mut desc = rustc_test::TestDesc::new(rustc_test::DynTestName(name));
            if should_panic {
                desc.should_panic = rustc_test::ShouldPanic::Yes
            }
            tests.push(rustc_test::TestDescAndFn { desc, testfn: run })
        };
        collect_data_url(&mut add_one);
    }
    rustc_test::test_main(&std::env::args().collect::<Vec<_>>(), tests)
}
