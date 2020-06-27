# jsonatr
JSON Artifact Translator

**Jsonatr** aims at providing intuitive ways of combining and translating between various software artifacts in JSON format. From the Unux perspective you can view `jsonatr` as lifting the `tr` translation command from strings to JSON datastructures.

Our primary motivation for the tool comes from the need to perform model-based testing of the **Tendermint** protocol implementations based on counterexamples as produced by our **Apalache** model checker. The counterexamples are produced in JSON, but they have very a minimalistic content. Real tests, on the other hand, are also JSON files, but need a lot of "meat", reflecting the data required by the implementation. Thus, the lightweight counterexamples need to be translated and expanded into the heavyweight tests: the ideal task for *Jsonatr*! As JSON-encoded data are ubiquitous in modern computing, we envision also numerous other applications for the tool.

*Jsonatr* works on the transformation specification encoded also in JSON. This spec describes where the input data should be taken from, and how it should be transformed to form the desired JSON output. Please check out these example transformation specs to gain the first impressions:
* [This spec](tests/support/store_with_jsonpath.json) extracts [this information](tests/support/store_with_jsonpath_output.json) from the [book store](tests/support/store.json) 
* [This spec](tests/support/counterexample_with_jsonpath.json) extracts [this information](tests/support/counterexample_with_jsonpath_output.json) from the [Apalache counterexample](tests/support/counterexample.json)
* [This spec](tests/support/counterexample_to_test.json) transforms the [Apalache counterexample](tests/support/counterexample.json) to [this test](tests/support/counterexample_to_test_output.json), which can be used for testing the Tendermint in Rust implementation. 

The current *Jsonatr* features include support for:

* direct embedding of JSON inputs into JSON output
* JsonPath expressions for accessing components of JSON inputs
* calling external transformers for transforming parts of the input
* mapping external/internal transformers over input JSON arrays

