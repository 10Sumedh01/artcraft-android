# ArtCraft Android Documentation Index

Welcome to the internal engineering documentation for the **ArtCraft Android Builder**.

## Available Documents

1. [**Architecture Specification (`ARCHITECTURE.md`)**](file:///home/sumedh/artcraft-android/docs/ARCHITECTURE.md)
   - High-level system architecture and execution domains.
   - Java Host (`SafActivity`) & Rust JNI Bridge.
   - Storage Access Framework (SAF) staging pipeline & POSIX compatibility.
   - Input, gestures, and IME soft keyboard engine.
   - Mobile graphics lifecycle, ANR handling, and font virtualization.
   - Build automation and packaging pipeline.

2. [**Learning Guide & Concepts Deep Dive (`LEARNING_GUIDE.md`)**](file:///home/sumedh/artcraft-android/docs/LEARNING_GUIDE.md)
   - Systems programming patterns: Safe JNI, thread attachment, and memory safety.
   - Desktop-to-mobile UI adaptation patterns (virtual modifiers, touch right-clicks).
   - Multi-touch signal processing & gesture state machines.
   - Overcoming modern Android Scoped Storage limitations.
   - The headless soft keyboard (IME) dilemma in native OpenGL/Vulkan apps.
   - Idempotent non-invasive source patching.
   - Hands-on exercises and experiments for learners.

3. [**CI/CD Workflow & GitHub Actions (`docs/ci-workflow.md`)**](file:///home/sumedh/artcraft-android/docs/ci-workflow.md)
   - Guide for GitHub Actions matrix builds, compilation caching, and APK release publishing.

4. [**Adapters Guide (`adapters/README.md`)**](file:///home/sumedh/artcraft-android/adapters/README.md)
   - Guide for creating and customizing app-specific adapters.
