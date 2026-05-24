# AI actions DSL

Так как framework ориентирован на AI-enabled business apps, AI-actions должны быть first-class.

```rust
ai action summarize_lesson(input: LessonTranscript) -> Summary {
  model: "soz-kz-600m"
  fallback: "gpt-5.5-thinking"
  temperature: 0.2
  audit: true
  pii: redact
  permission: "lessons.ai.summarize"
}
```

## Template usage

```html
<button on:click="summarize_lesson(current_lesson.transcript)">
  Generate summary
</button>
```

## Автоматическая поддержка

AI action должен автоматически поддерживать:

* prompt template;
* model routing;
* audit;
* tenant budget;
* rate limit;
* PII redaction;
* output schema validation;
* retry/fallback;
* trace log.
