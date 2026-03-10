import { ActionButton, ChoiceDropdown, ModelFeatureBadge, ModelPickerPreview, StatusChip } from "../components/common";
import {
  BoltIcon,
  CheckIcon,
  ClockIcon,
  CpuIcon,
  DownloadIcon,
  ExternalIcon,
  SparkIcon,
  TrashIcon,
} from "../components/icons";
import { describeHardwareFit, formatModelAudioLimit, formatModelSizeLabel, modelFeatureItems } from "../lib/modelCatalog";
import { formatSystemProfile } from "../lib/utils";
import type {
  ButtonFeedbackState,
  ChoiceOption,
  LivePreviewModel,
  ModelRow,
  Snapshot,
} from "../types";

function featureIcon(icon: "spark" | "cpu" | "bolt" | "users" | "clock") {
  switch (icon) {
    case "cpu":
      return <CpuIcon className="small-icon" />;
    case "bolt":
      return <BoltIcon className="small-icon" />;
    case "clock":
      return <ClockIcon className="small-icon" />;
    case "spark":
    default:
      return <SparkIcon className="small-icon" />;
  }
}

function BatchModelRow({
  row,
  snapshot,
  buttonFeedback,
  onActivateModel,
  onDownloadCatalogModel,
  onRemoveCatalogModel,
  onOpenModelReference,
}: {
  row: ModelRow;
  snapshot: Snapshot;
  buttonFeedback: Record<string, ButtonFeedbackState>;
  onActivateModel: (row: ModelRow) => void | Promise<void>;
  onDownloadCatalogModel: (row: ModelRow) => void | Promise<void>;
  onRemoveCatalogModel: (row: ModelRow) => void | Promise<void>;
  onOpenModelReference: (row: ModelRow) => void | Promise<void>;
}) {
  const fit = describeHardwareFit(row, snapshot.systemProfile);

  return (
    <article className={`model-list-row ${row.active ? "model-list-row-active" : ""}`}>
      <div className="model-list-main">
        <div className="model-list-head">
          <strong>{row.name}</strong>
          {row.active ? <StatusChip label="Active" tone="success" /> : null}
          {!row.active && row.state === "ready" ? (
            <StatusChip label="Ready" tone="accent" />
          ) : null}
          {row.state === "downloadable" ? (
            <StatusChip label="Download" tone="muted" />
          ) : null}
        </div>

        <div className="model-entry-meta">
          <span>{row.footprint}</span>
          <span>{row.architecture}</span>
          <span>{formatModelSizeLabel(row)}</span>
          <span>{formatModelAudioLimit(row)}</span>
        </div>

        <p className="model-list-note">{row.note}</p>

        <div className="model-feature-list">
          {modelFeatureItems(row).map((feature) => (
            <ModelFeatureBadge
              key={`${row.id}-${feature.id}`}
              icon={featureIcon(feature.icon)}
              label={feature.label}
            />
          ))}
        </div>
      </div>

      <div className="model-list-side">
        <span className="model-list-stat">{fit.label}</span>
        <div className="model-row-actions">
          {row.active ? null : row.selectable ? (
            <button className="secondary small" onClick={() => void onActivateModel(row)}>
              Use
            </button>
          ) : null}
          {row.supportsDownload && row.state !== "ready" ? (
            <ActionButton
              className="secondary small"
              state={buttonFeedback[`model-download:${row.id}`]}
              idleLabel="Download"
              workingLabel="Downloading"
              doneLabel="Downloaded"
              idleIcon={<DownloadIcon className="small-icon" />}
              workingIcon={<DownloadIcon className="small-icon" />}
              doneIcon={<CheckIcon className="small-icon" />}
              onClick={() => onDownloadCatalogModel(row)}
              iconOnly
            />
          ) : null}
          {row.managed ? (
            <ActionButton
              className="secondary small"
              state={buttonFeedback[`model-remove:${row.id}`]}
              idleLabel="Delete downloaded model"
              workingLabel="Deleting"
              doneLabel="Deleted"
              idleIcon={<TrashIcon className="small-icon" />}
              doneIcon={<CheckIcon className="small-icon" />}
              onClick={() => onRemoveCatalogModel(row)}
              iconOnly
            />
          ) : null}
          {row.hfUrl ? (
            <button
              className="secondary small icon-only-button"
              onClick={() => void onOpenModelReference(row)}
              aria-label="Open Hugging Face"
              title="Open Hugging Face"
            >
              <ExternalIcon className="small-icon" />
            </button>
          ) : null}
        </div>
      </div>
    </article>
  );
}

function StreamingModelRow({
  row,
  snapshot,
  buttonFeedback,
  effectiveLivePreviewModelId,
  explicitLivePreviewModel,
  onDownloadCatalogModel,
  onRemoveCatalogModel,
  onOpenModelReference,
}: {
  row: ModelRow;
  snapshot: Snapshot;
  buttonFeedback: Record<string, ButtonFeedbackState>;
  effectiveLivePreviewModelId: string | null;
  explicitLivePreviewModel: LivePreviewModel;
  onDownloadCatalogModel: (row: ModelRow) => void | Promise<void>;
  onRemoveCatalogModel: (row: ModelRow) => void | Promise<void>;
  onOpenModelReference: (row: ModelRow) => void | Promise<void>;
}) {
  const fit = describeHardwareFit(row, snapshot.systemProfile);
  const isEffective = effectiveLivePreviewModelId === row.id;
  const isExplicit = explicitLivePreviewModel !== "auto" && explicitLivePreviewModel === row.id;

  return (
    <article className={`model-list-row ${isEffective ? "model-list-row-active" : ""}`}>
      <div className="model-list-main">
        <div className="model-list-head">
          <strong>{row.name}</strong>
          {isExplicit ? <StatusChip label="Selected" tone="success" /> : null}
          {!isExplicit && isEffective ? <StatusChip label="Auto" tone="accent" /> : null}
          {!isEffective && row.state === "ready" ? (
            <StatusChip label="Installed" tone="accent" />
          ) : null}
          {row.state === "downloadable" ? (
            <StatusChip label="Download" tone="muted" />
          ) : null}
        </div>

        <div className="model-entry-meta">
          <span>{row.footprint}</span>
          <span>{row.architecture}</span>
          <span>{formatModelSizeLabel(row)}</span>
          <span>{row.bestFor}</span>
        </div>

        <p className="model-list-note">{row.note}</p>

        <div className="model-feature-list">
          {modelFeatureItems(row).map((feature) => (
            <ModelFeatureBadge
              key={`${row.id}-${feature.id}`}
              icon={featureIcon(feature.icon)}
              label={feature.label}
            />
          ))}
        </div>
      </div>

      <div className="model-list-side">
        <span className="model-list-stat">{fit.label}</span>
        <div className="model-row-actions">
          {row.supportsDownload && row.state !== "ready" ? (
            <ActionButton
              className="secondary small"
              state={buttonFeedback[`model-download:${row.id}`]}
              idleLabel="Download"
              workingLabel="Downloading"
              doneLabel="Downloaded"
              idleIcon={<DownloadIcon className="small-icon" />}
              workingIcon={<DownloadIcon className="small-icon" />}
              doneIcon={<CheckIcon className="small-icon" />}
              onClick={() => onDownloadCatalogModel(row)}
              iconOnly
            />
          ) : null}
          {row.managed ? (
            <ActionButton
              className="secondary small"
              state={buttonFeedback[`model-remove:${row.id}`]}
              idleLabel="Delete downloaded model"
              workingLabel="Deleting"
              doneLabel="Deleted"
              idleIcon={<TrashIcon className="small-icon" />}
              doneIcon={<CheckIcon className="small-icon" />}
              onClick={() => onRemoveCatalogModel(row)}
              iconOnly
            />
          ) : null}
          {row.hfUrl ? (
            <button
              className="secondary small icon-only-button"
              onClick={() => void onOpenModelReference(row)}
              aria-label="Open Hugging Face"
              title="Open Hugging Face"
            >
              <ExternalIcon className="small-icon" />
            </button>
          ) : null}
        </div>
      </div>
    </article>
  );
}

export function ModelsSection({
  snapshot,
  batchModels,
  streamingModels,
  activeModel,
  activeReadyModelId,
  readyModelOptions,
  livePreviewModel,
  livePreviewOptions,
  effectiveLivePreviewModelId,
  buttonFeedback,
  onChooseDefaultModel,
  onChooseLivePreviewModel,
  onActivateModel,
  onDownloadCatalogModel,
  onRemoveCatalogModel,
  onOpenModelReference,
}: {
  snapshot: Snapshot;
  batchModels: ModelRow[];
  streamingModels: ModelRow[];
  activeModel: ModelRow | null;
  activeReadyModelId: string;
  readyModelOptions: ChoiceOption[];
  livePreviewModel: LivePreviewModel;
  livePreviewOptions: ChoiceOption[];
  effectiveLivePreviewModelId: string | null;
  buttonFeedback: Record<string, ButtonFeedbackState>;
  onChooseDefaultModel: (value: string) => void | Promise<void>;
  onChooseLivePreviewModel: (value: LivePreviewModel) => void | Promise<void>;
  onActivateModel: (row: ModelRow) => void | Promise<void>;
  onDownloadCatalogModel: (row: ModelRow) => void | Promise<void>;
  onRemoveCatalogModel: (row: ModelRow) => void | Promise<void>;
  onOpenModelReference: (row: ModelRow) => void | Promise<void>;
}) {
  const directmlReady = snapshot.systemProfile.directmlAvailable;

  return (
    <section className="model-page">
      <article className="surface model-decision-surface">
        <div className="model-library-head">
          <div className="model-library-copy">
            <span className="surface-title-label">Speech models</span>
            <p>Choose one batch model for final transcription, and one streaming model for live preview.</p>
          </div>
          <div className="model-selector-meta">
            <StatusChip
              label={activeModel ? `${activeModel.name} active` : "No batch model"}
              tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
            />
            {directmlReady ? <StatusChip label="DirectML ready" tone="accent" /> : null}
            <span className="model-selector-footnote">
              {formatSystemProfile(snapshot.systemProfile)}
            </span>
          </div>
        </div>

        <div className="model-decision-grid">
          <div className="model-decision-card">
            <div className="model-decision-copy">
              <strong>Final transcription</strong>
              <span>Used for microphone dictation, auto-paste, and file transcription.</span>
            </div>
            <ChoiceDropdown
              label="Active batch model"
              value={activeReadyModelId}
              options={readyModelOptions}
              placeholder="Download a batch model"
              renderPreview={(value) => {
                const row = batchModels.find((candidate) => candidate.id === value);
                return (
                  <ModelPickerPreview
                    active={Boolean(row?.active)}
                    selectable={Boolean(row?.selectable)}
                  />
                );
              }}
              onChange={(value) => {
                void onChooseDefaultModel(value);
              }}
            />
            <p className="model-decision-note">
              TDT and CTC are batch models with a soft per-pass limit of about five minutes.
            </p>
          </div>

          <div className="model-decision-card">
            <div className="model-decision-copy">
              <strong>Live transcription</strong>
              <span>Used only for the draft live preview shown while you speak.</span>
            </div>
            <ChoiceDropdown
              label="Active live model"
              value={livePreviewModel}
              options={livePreviewOptions}
              onChange={(value) => {
                void onChooseLivePreviewModel(value as LivePreviewModel);
              }}
            />
            <p className="model-decision-note">
              Auto prefers Nemotron when it is installed, otherwise Realtime EOU.
            </p>
          </div>
        </div>
      </article>

      <div className="model-groups">
        <article className="surface model-group-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Batch models</span>
            </div>
          </div>

          <div className="model-group-copy">
            <p>Pick the TDT or CTC variant you want Transcribed to use for final text.</p>
          </div>

          <div className="model-list">
            {batchModels.map((row) => (
              <BatchModelRow
                key={row.id}
                row={row}
                snapshot={snapshot}
                buttonFeedback={buttonFeedback}
                onActivateModel={onActivateModel}
                onDownloadCatalogModel={onDownloadCatalogModel}
                onRemoveCatalogModel={onRemoveCatalogModel}
                onOpenModelReference={onOpenModelReference}
              />
            ))}
          </div>
        </article>

        <article className="surface model-group-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Live preview models</span>
            </div>
          </div>

          <div className="model-group-copy">
            <p>These models only affect the live preview text. Final pasted text still comes from the active batch model.</p>
          </div>

          <div className="model-list">
            {streamingModels.map((row) => (
              <StreamingModelRow
                key={row.id}
                row={row}
                snapshot={snapshot}
                buttonFeedback={buttonFeedback}
                effectiveLivePreviewModelId={effectiveLivePreviewModelId}
                explicitLivePreviewModel={livePreviewModel}
                onDownloadCatalogModel={onDownloadCatalogModel}
                onRemoveCatalogModel={onRemoveCatalogModel}
                onOpenModelReference={onOpenModelReference}
              />
            ))}
          </div>
        </article>
      </div>
    </section>
  );
}
