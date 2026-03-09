import { CheckIcon, ExternalIcon, FolderIcon } from "../components/icons";

export function AboutSection() {
  return (
    <section className="compact-grid-three">
      <article className="surface info-tile">
        <CheckIcon className="tile-icon-svg" />
        <strong>Local only</strong>
        <span>Audio, paste, and history stay on-device.</span>
      </article>
      <article className="surface info-tile">
        <FolderIcon className="tile-icon-svg" />
        <strong>Tray-first</strong>
        <span>Close hides the window and keeps hotkeys alive.</span>
      </article>
      <article className="surface info-tile">
        <ExternalIcon className="tile-icon-svg" />
        <strong>Next</strong>
        <span>Better live preview, more Parakeet variants, vector search.</span>
      </article>
    </section>
  );
}
