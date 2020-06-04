# jsonatr
JSON Artifact Translator

**Jsonatr** aims at providing intuitive ways of combining and translating between various software artifacts in JSON format. From the Unux perspective you can view `jsonatr` as lifting the `tr` translation command from strings to JSON datastructures.

Our primary motivation for the tool comes from the need to perform model-based testing of the **Tendermint** protocol implementations based on counterexamples as produced by our **Apalache** model checker. The counterexamples are produced in JSON, but they have very a minimalistic content. Real tests, on the other hand, are also JSON files, but need a lot of "meat", reflecting the data required by the implementation. Thus, the lightweight counterexamples need to be translated and expanded into the heavyweight tests: the ideal task for *Jsonatr*! As JSON-encoded data are ubiquitous in modern computing, we envision also numerous other applications for the tool.

*Jsonatr* will work on the transformation specification encoded also in JSON. This spec will describe where the input data should be taken from, and how it should be transformed to form the desired JSON output. The planned features for *Jsonatr* include support for:

* direct embedding of JSON inputs into JSON output
* JsonPath expressions for accessing components of JSON inputs
* calling external transformers for transforming parts of the input
* mapping transformers over input JSON arrays

