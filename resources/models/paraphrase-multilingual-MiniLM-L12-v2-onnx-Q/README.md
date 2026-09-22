---
license: apache-2.0
pipeline_tag: sentence-similarity
---

Quantized ONNX port of [sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2](https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2) for text classification and similarity searches.

### Usage

Here's an example of performing inference using the model with [FastEmbed](https://github.com/qdrant/fastembed).

```py
from fastembed import TextEmbedding

documents = [
    "You should stay, study and sprint.",
    "History can only prepare us to be surprised yet again.",
]

model = TextEmbedding(model_name="sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")
embeddings = list(model.embed(documents))

# [
#     array([1.96449570e-02, 1.60677675e-02, 4.10149433e-02...]),
#     array([-1.56669170e-02, -1.66313536e-02, -6.84525725e-03...])
# ]
```