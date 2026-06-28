---
id: ctx-369418
created: 2026-06-26T17:02:16.373869669Z
status: current
concerns:
- flutter-bedrock-voice-agent-plan
scope:
  paths: []
  components: []
superseded_by: []
---
### flutter-bedrock-voice-agent-plan [r2]

A Flutter voice agent app was discussed targeting Amazon Bedrock. The intended stack is Flutter + langchain_dart (langchain_aws package) + Amazon Bedrock (Claude 3.5 Sonnet or similar) + speech_to_text + flutter_tts.

The architecture is: Microphone → STT → LangChain Agent (Bedrock/Claude) → Tool Executor → TTS → Speaker. Tool definitions use LangChain Tool objects. The agent decides when to call them. This gives the same capability as terminal agent harnesses (goose, codecs) but running on-device in a Flutter app.

This work was discussed but not started in this session. It is deferred. The ctx MCP server implementation took priority.

Mobile agent harness landscape surfaced during discussion:
- langchain_dart: best fit, has Bedrock provider, built-in agent executor, tool calling
- MLC LLM: on-device quantized inference (LLaMA, Mistral)
- Google AI Edge / MediaPipe LLM: on-device Gemma
- Semantic Kernel (.NET MAUI): Microsoft agent harness for mobile
- Vercel AI SDK: React Native option with streaming and tool calls
