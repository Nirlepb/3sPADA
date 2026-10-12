# Tamper-Evident Audit Ledger for Financial Transactions Using an LSM Tree with Merkle-Verified SSTables

## Overview
This repository contains the implementation of a tamper-evident audit ledger designed specifically for financial transactions. The core architecture leverages a Log-Structured Merge (LSM) Tree to handle high-throughput data operations, integrated with Merkle-Verified Sorted String Tables (SSTables) to guarantee cryptographic data integrity and immutability.

## Team Members
| Name | Roll Number |
| :--- | :--- |
| BODDUPALLLY NIRLEP | CB.SC.U4CSE25313 |
| Reshvanth Yeddla | CB.SC.U4CSE25347 |
| Tharunn D | CB.SC.U4CSE25359 |
| Vijay Raghava Reddy | CB.SC.U4CSE25363 |

## System Architecture
Our ledger is built on three primary components to balance high-speed ingestion with strict cryptographic security:
1. **MemTable (In-Memory Buffer):** Buffers incoming financial transactions (e.g., transfers, deposits). Implemented as a self-balancing binary search tree.
2. **Write-Ahead Log (WAL):** Ensures absolute durability. Every transaction is appended to the WAL before being acknowledged, preventing data loss during system crashes.
3. **Merkle-Verified SSTables (On-Disk):** When the MemTable reaches capacity, it flushes to disk as an immutable SSTable. A Merkle Tree is dynamically constructed over the transaction blocks, and the Merkle Root is saved. Any silent data corruption or malicious tampering alters the hash, failing the verification check.

## Prerequisites
* GCC/G++ Compiler (C++17 or higher) or JDK 17+
* OpenSSL (for SHA-256 hashing)
* CMake (for building the project)

## Installation and Build
1. Clone the repository:
   ```bash
   git clone [https://github.com/your-username/your-repo-name.git](https://github.com/your-username/your-repo-name.git)
   cd your-repo-name
