# jsonart
JSON Artifact Translator

**Jsonart** aims at providing intuitive ways of combining and translating between various software artifacts in JSON format. From the Unux perspective you can view `jsonart` as lifting the `tr` translation command from strings to JSON datastructures.

Our primary motivation for the tool comes from the need to perform model-based testing of the **Tendermint** protocol implementations based on counterexamples as produced by our **Apalache** model checker. The counterexamples are produced in JSON, but they have very a minimalistic content. Real tests, on the other hand, are also JSON files, but need a lot of "meat", reflecting the data required by the implementation. Thus, the lightweight counterexamples need to be translated and expanded into the heavyweight tests: the ideal task for *Jsonart*! As JSON-encoded data are ubiquitous in modern computing, we envision also numerous other applications for the tool.

*Jsonart* works on a transformation specification encoded also in JSON. The transformation spec describes where the input data should be taken from, and how they should be transformed to form the desired JSON output. Please check out these example transformation specs to gain the first impressions:
* [This spec](tests/support/store_with_jsonpath.json) extracts [this information](tests/support/store_with_jsonpath_output.json) from the [book store](tests/support/store.json) 
* [This spec](tests/support/counterexample_with_jsonpath.json) extracts [this information](tests/support/counterexample_with_jsonpath_output.json) from the [Apalache counterexample](tests/support/counterexample.json)
* [This spec](tests/support/apalache_counterexample_to_lightclient_test.json) transforms the [Apalache counterexample](tests/support/apalache_counterexample.json) into [this test](tests/support/apalache_counterexample_to_lightclient_test_output.json), which can be used for testing the Tendermint in Rust implementation. 

The current *Jsonart* features include support for:

* direct embedding of JSON inputs into JSON output
* JsonPath expressions for accessing components of JSON inputs
* calling external transformers for transforming parts of the input
* mapping external/internal transformers over input JSON arrays

## License

Copyright © 2020 Informal Systems

Licensed under the Apache License, Version 2.0 (the "License");
you may not use the files in this repository except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
