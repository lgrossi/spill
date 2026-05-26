import { AppChrome, Btn } from "../../components/spill-ui";
import { NewBoardForm, type TemplateId } from "./new-board-form";

export default async function NewRetroPage({
  searchParams,
}: {
  searchParams: Promise<{ template?: string }>;
}) {
  const params = await searchParams;
  const selectedTemplate = normalizeTemplate(params.template);

  return (
    <AppChrome
      actions={
        <>
          <Btn href="/" kind="secondary">cancel</Btn>
          <Btn form="new-board-form" kind="primary" type="submit">pin it up</Btn>
        </>
      }
      subtitle="set it up. takes 20 seconds"
      title="New board"
    >
      <NewBoardForm selectedTemplate={selectedTemplate} />
    </AppChrome>
  );
}

function normalizeTemplate(template: string | undefined): TemplateId {
  if (template === "4ls" || template === "custom" || template === "standard") return template;
  return "standard";
}
