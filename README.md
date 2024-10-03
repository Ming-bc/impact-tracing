# Implementation of Impact Tracing

*This repository contains the implementation and evaluation of impact tracing, a message traceback scheme for end-to-end encrypted messaging systems.*

**NDSS Submission #980:** Impact tracing: Identifying the Culprit of Misinformation for Encrypted Messaging Systems

## Overview

The implementation mainly consists of the following modules:

- [src](src):  Rust implementation of our scheme.
  - [db](src/db) & [tool](src/tool): Implement database and crypto operations.
  - [message](src/message) & [trace](src/trace): Implement algorithms for sending/processing/receiving and tracing a message, respectively.
  - [simulation](src/simulation): Implement susceptible-infected-recovered (SIR) model and our decoding function.
  - [rwc_eval](src/rwc_eval): Evaluate the runtime of tracing in real-world datasets.
  - [analysis](src/analysis): Evaluate the utility and privacy under specified metrics.
- [python](python): Python implementation of auxiliary functions for evaluation.
  - [influenence](python/influence): Implement the k-shell decomposition algorithm.

## Installation

The implementation is evaluated on a PC running Ubuntu OS, with Rust and Python3 environment.

Install necessary packets:

```
sudo apt-get install build-essential graphviz graphviz-dev
pip install networkx pydot numpy pygraphviz
```

Switch Rust channel to nightly for benchmark:

```
rustup toolchain install nightly
rustup override set nightly
```

Install and run three Redis databases in [Docker](https://docs.docker.com/engine/install/ubuntu/), where the ip addresses and ports are configured in [.env](.env):

```
docker pull redis:latest
docker run -itd --name db_ik -p 6400:6379 redis
docker run -itd --name db_nbr -p 6401:6379 redis
docker run -itd --name db_tag -p 6402:6379 redis
```

Run test to ensure that the databases are properly connected:

```
cargo test redis_is_open
```

Run [init.sh](shell/init.sh) to create folders for storing temporary files when generating simulated graphs:

```
bash shell/init.sh
```

## Running

Run benchmark for runtime:

```
cargo bench message
cargo test -- --nocapture test_plt_proc
cargo test -- --nocapture test_trace_time
```

Run benchmark for utility and privacy:

```
bash shell/gen_thd_fpr.sh
```

The experimental results can be found in [output](python/outputs/).