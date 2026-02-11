# 📋 AegisNode Roadmap

This document outlines the development roadmap and future plans for the AegisNode project.

## 🌟 Upcoming Features
- [ ] **Data Rehydration**: Mechanism to restore redacted data in the response or for authorized users.
- [ ] **Response Scanning**: Ability to scan and filter responses from LLMs (preventing harmful output).
- [ ] **NLP-based Detection**: Integration of local NLP models (like BERT/NER) for more accurate entity detection compared to Regex.
- [ ] **Audit Logging**: Centralized logging system for security audit and compliance purposes.

## 🚀 Performance Enhancements
- [ ] **Streaming Support**: Support for *streaming* request/response processing to reduce latency on large payloads.
- [ ] **Caching Layer**: Implementation of cache for common responses to save API token costs.

## 🔌 Additional Integrations
- [ ] **Azure OpenAI Adapter**: Support for special authentication for Azure Managed Identity.
- [ ] **Local LLM Fallback**: Option to fallback to local models (Llama/Mistral) if highly sensitive data is detected.

## 📚 Documentation
- [ ] **API Reference**: Swagger/OpenAPI documentation for proxy endpoints.
- [ ] **Security Hardening Guide**: Guide for secure production deployment configuration.

## 🐛 Known Issues
- [ ] Handling of *chunked transfer encoding* is still experimental for some providers.
