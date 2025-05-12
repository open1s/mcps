# MCPS - Model Context Protocol for Rust

A Rust implementation of Anthropic's [Model Context Protocol (MCP)](https://modelcontextprotocol.io/specification/2025-03-26), Specification defined at [repo](https://github.com/modelcontextprotocol/modelcontextprotocol), which is an open standard for connecting AI assistants to data sources and tools.

This project is inspired by mcpr at https://github.com/conikeec/mcpr. some of the code is copied from there. the goal here is to provide new transport mechanisms and to make the code more idiomatic for Rust. and to remove dependency on external tokio etc. 
