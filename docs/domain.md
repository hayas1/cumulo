# cumulo's domain model

The bipartite-graph model cumulo works with, and its vocabulary. It records why the model is shaped this way.

## Core model: a bipartite graph

There are only two kinds of things.

- **Resource**: the thing you want to reach (a console, a portal…).
- **Category**: the vocabulary used to classify resources.

A resource is classified along **several independent aspects** — e.g. *which platform* × *which environment* × *which team*. These aspects are orthogonal.

Forcing orthogonal aspects into a single tree bakes every combination of aspects into the tree's depth and branching, which explodes combinatorially. So cumulo keeps **a separate tree per aspect**, and a resource picks one value from each aspect's tree. This is why Resource and Category form a **bipartite graph**.

Both the resource side and the category side are **forests** (collections of trees) that express `is-a` through `parent` links.

| Term | What it means |
|---|---|
| **Resource** | The thing you want to reach. A node of the Catalog (forest). |
| **Category** | The vocabulary for classification. A node of the Taxonomy (forest). |
| **Catalog** | The forest on the resource side. |
| **Taxonomy** | The forest on the category side. |
| **Bipartite** | The whole spanning Resource and Category. The application state itself. |

## Axis and value

- **Axis**: the **root** of a category tree. It corresponds to a facet — an aspect of classification. Examples: `platform`, `env`, `team`.
- **Value**: a node belonging to some axis's tree. A resource's classification is expressed by these values.
  - **Every node, including a root, can be a value** — they are treated uniformly. Both `gcp` and the `bigquery` under it are values.
  - A resource holds only a list of values, not the axes; which axis a value belongs to is found by walking up to the root of that value's tree. A value's axis is thus fixed entirely by the tree structure.
- **One value per axis**: a resource holds at most one value per axis.
  - Why: an axis is "one orthogonal aspect," so a resource's position along that aspect should be a single value (`platform` being both `gcp` and `aws` at once contradicts what an aspect means).

## Semantics of filtering (association)

Filtering is expressed as a mapping of "axis → chosen value."

- **AND across axes**: adding aspects narrows the result. "`platform=bigquery` and `env=prod`." This is exactly the point — layering a few clues to converge quickly on one resource.
- **Within an axis, an ancestor subsumes its descendants (ancestry match)**: choosing `gcp` also includes resources holding `bigquery`, which sits under it. This mirrors the **granularity of association** — higher categories are vaguer clues, lower ones more specific.

## Domain guidelines (not enforced by code)

- **Keep category trees shallow.** Deep trees increase the clues needed for recall and undermine faceting's strength of "narrowing with few clues."
- **A resource's URL need not be derivable from its categories.** A category is a clue for association, not a rule for generating URLs.
- The resource side can also form a forest (parent/child), but the main mechanism is filtering by category, and the resource hierarchy is secondary.
