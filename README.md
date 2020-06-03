# jonatr
JSON Artifact Translator

**Jonatr** (JsON Artifact TRanslator) aims at providing intuitive ways of combining and translating between various software artifacts in JSON format. From Linux perspective you can view `jonatr` it as lifting of the Linux `tr` command to JSON datastructures.

Our primary motivation for the tool comes from the need to do model-based testing of the **Tendermint** protocol implementations based on counterexamples as produced by our **Apalache** model checker. The counterexamples are produced in JSON, but they have very a minimalistic contents. Real tests, on the other hand, are also JSON files, but need a lot of "meat", reflecting the data required by the implementation. Thus, the lightweight counterexamples need to be translated and expanded into the heavyweight tests: the ideal task for *Jonatr*! As JSON-encoded data are ubiquitous in modern computing, we envision also numerous other applications for the tool.

*Jonatr* will work on the transformation specification encoded also in JSON. This spec will describe where the input data is taken from, and how it should be transformed to form the desired JSON output. The planned features for *Jonatr* include support for:

* direct embedding of JSON inputs into JSON output
* JsonPath expressions for accessing components of JSON inputs
* calling external transformers for trasforming parts of the input
* mapping transformers over input JSON arrays

