# Пример: school lesson tracker

```html
<route>
  path: "/lessons/[id]"
  layout: "school"
  auth: required
  tenant: required
  permission: "lessons.read"
</route>

<script lang="rust">
  prop id: Uuid

  state lesson: Lesson
  state transcript: String = ""
  state ai_summary: Option<LessonSummary> = None
  state saving: bool = false

  load async fn load(ctx: Ctx) {
    lesson = LessonRepo::find(ctx.tenant_id, id).await?;
    transcript = lesson.transcript.clone();
  }

  #[permission("lessons.update")]
  #[audit("lesson.status.changed")]
  action async fn set_status(status: LessonStatus, ctx: Ctx) {
    lesson.status = status;
    LessonRepo::set_status(ctx.tenant_id, id, status).await?;
  }

  ai action summarize() -> LessonSummary {
    model: "soz-kz-600m"
    fallback: "gpt-5.5-thinking"
    input: transcript
    pii: redact
    permission: "lessons.ai.summarize"
    audit: true
  }
</script>

<template>
  <Page title={lesson.title}>
    <StatusBar status={lesson.status} />

    <ButtonGroup>
      <Button on:click="set_status('planned')">Planned</Button>
      <Button on:click="set_status('in_progress')">In progress</Button>
      <Button on:click="set_status('done')">Done</Button>
    </ButtonGroup>

    <TextArea bind:value="transcript" />

    <Button on:click="summarize">Generate AI summary</Button>

    {#if ai_summary}
      <Card>
        <h2>AI Summary</h2>
        <p>{ai_summary.text}</p>
      </Card>
    {/if}
  </Page>
</template>
```
