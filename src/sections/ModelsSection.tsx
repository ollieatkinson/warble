import { modelFilters } from "../constants";
import { ActionButton, ChoiceDropdown, ModelFeatureBadge, ModelPickerPreview, ScoreMeter, StatusChip } from "../components/common";
import {
  BoltIcon,
  CheckIcon,
  ClockIcon,
  CpuIcon,
  DownloadIcon,
  ExternalIcon,
  FolderIcon,
  SearchIcon,
  SparkIcon,
  TrashIcon,
  UsersIcon,
} from "../components/icons";
import {
  describeHardwareFit,
  formatModelSizeLabel,
  modelAccuracyScore,
  modelFeatureItems,
  modelSpeedScore,
} from "../lib/modelCatalog";
import { formatBytes, formatSystemProfile } from "../lib/utils";
import type {
  ButtonFeedbackState,
  ChoiceOption,
  ModelFilter,
  ModelRow,
  Snapshot,
} from "../types";

function featureIcon(icon: "spark" | "cpu" | "bolt" | "users" | "clock") {
  switch (icon) {
    case "cpu":
      return <CpuIcon className="small-icon" />;
    case "bolt":
      return <BoltIcon className="small-icon" />;
    case "users":
      return <UsersIcon className="small-icon" />;
    case "clock":
      return <ClockIcon className="small-icon" />;
    case "spark":
    default:
      return <SparkIcon className="small-icon" />;
  }
}

export function ModelsSection({
  snapshot,
  modelRows,
  filteredModels,
  modelQuery,
  modelFilter,
  selectedModel,
  selectedModelFit,
  selectedModelMeta,
  resolvedSelectedModelId,
  activeModel,
  activeReadyModelId,
  readyModelOptions,
  buttonFeedback,
  onSetModelQuery,
  onSetModelFilter,
  onSelectModel,
  onChooseDefaultModel,
  onActivateModel,
  onLinkCatalogModel,
  onDownloadCatalogModel,
  onRemoveCatalogModel,
  onOpenModelReference,
  onOpenModelArtifact,
}: {
  snapshot: Snapshot;
  modelRows: ModelRow[];
  filteredModels: ModelRow[];
  modelQuery: string;
  modelFilter: ModelFilter;
  selectedModel: ModelRow | undefined;
  selectedModelFit:
    | {
        label: string;
        tone: "success" | "warning" | "muted" | "accent";
        detail: string;
      }
    | null;
  selectedModelMeta: Array<[string, string]>;
  resolvedSelectedModelId: string;
  activeModel: ModelRow | null;
  activeReadyModelId: string;
  readyModelOptions: ChoiceOption[];
  buttonFeedback: Record<string, ButtonFeedbackState>;
  onSetModelQuery: (value: string) => void;
  onSetModelFilter: (value: ModelFilter) => void;
  onSelectModel: (row: ModelRow) => void;
  onChooseDefaultModel: (value: string) => void | Promise<void>;
  onActivateModel: (row: ModelRow) => void | Promise<void>;
  onLinkCatalogModel: (row: ModelRow) => void | Promise<void>;
  onDownloadCatalogModel: (row: ModelRow) => void | Promise<void>;
  onRemoveCatalogModel: (row: ModelRow) => void | Promise<void>;
  onOpenModelReference: (row: ModelRow) => void | Promise<void>;
  onOpenModelArtifact: (row: ModelRow) => void | Promise<void>;
}) {
  return (
    <section className="surface model-library-surface">
      <div className="model-library-head">
        <div className="model-library-copy">
          <span className="surface-title-label">Speech models</span>
          <p>Compare NVIDIA speech runtimes, DirectML-ready streaming add-ons, and speaker-aware pipelines.</p>
        </div>
      </div>

      <div className="model-selector-row">
        <div className="model-selector-card">
          <ChoiceDropdown
            label="Default speech model"
            value={activeReadyModelId}
            options={readyModelOptions}
            placeholder="Download a model"
            renderPreview={(value) => {
              const row = modelRows.find((candidate) => candidate.id === value);
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
        </div>

        <div className="model-selector-meta">
          <StatusChip
            label={activeModel ? `${activeModel.name} active` : "No active model"}
            tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
          />
          {snapshot.systemProfile.directmlAvailable ? (
            <StatusChip label="DirectML ready" tone="accent" />
          ) : null}
          {selectedModelFit ? (
            <StatusChip label={selectedModelFit.label} tone={selectedModelFit.tone} />
          ) : null}
          <span className="model-selector-footnote">
            {formatSystemProfile(snapshot.systemProfile)}
          </span>
        </div>
      </div>

      <div className="toolbar model-toolbar">
        <label className="search-field">
          <SearchIcon className="search-icon" />
          <input
            type="search"
            value={modelQuery}
            onChange={(event) => onSetModelQuery(event.currentTarget.value)}
            placeholder="Search NVIDIA speech models"
          />
        </label>

        <div className="segmented">
          {modelFilters.map((filter) => (
            <button
              key={filter.id}
              type="button"
              className={`segment ${modelFilter === filter.id ? "segment-active" : ""}`}
              onClick={() => onSetModelFilter(filter.id)}
            >
              {filter.label}
            </button>
          ))}
        </div>
      </div>

      <div className="table-shell model-library-table-shell">
        <table className="model-table model-library-table">
          <thead>
            <tr>
              <th />
              <th>Model</th>
              <th>Capabilities</th>
              <th>Speed</th>
              <th>Quality</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {filteredModels.map((row) => {
              const hardwareFit = describeHardwareFit(row, snapshot.systemProfile);

              return (
                <tr
                  key={row.id}
                  className={[
                    "model-row",
                    row.id === resolvedSelectedModelId ? "model-row-selected" : "",
                    row.active ? "model-row-active" : "",
                    row.state === "planned" || row.state === "incomplete"
                      ? "model-row-dim"
                      : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => onSelectModel(row)}
                >
                  <td className="model-radio-cell">
                    <span
                      className={[
                        "model-radio",
                        row.id === resolvedSelectedModelId ? "model-radio-selected" : "",
                        row.active ? "model-radio-active" : "",
                      ]
                        .filter(Boolean)
                        .join(" ")}
                    />
                  </td>
                  <td className="model-main-cell">
                    <div className="model-entry">
                      <div className="model-entry-head">
                        <strong>{row.name}</strong>
                        {row.active ? <span className="model-inline-pill">Default</span> : null}
                        {!row.selectable && row.state === "ready" ? (
                          <span className="model-inline-pill model-inline-pill-accent">Installed</span>
                        ) : null}
                      </div>
                      <div className="model-entry-meta">
                        <span>{row.provider}</span>
                        <span>{row.speechMode}</span>
                        <span>{row.architecture}</span>
                        <span>{formatModelSizeLabel(row)}</span>
                      </div>
                    </div>
                  </td>
                  <td>
                    <div className="model-feature-list">
                      {modelFeatureItems(row).map((feature) => (
                        <ModelFeatureBadge
                          key={`${row.id}-${feature.id}`}
                          icon={featureIcon(feature.icon)}
                          label={feature.label}
                        />
                      ))}
                    </div>
                  </td>
                  <td>
                    <ScoreMeter value={modelSpeedScore(row)} kind="speed" />
                  </td>
                  <td>
                    <ScoreMeter value={modelAccuracyScore(row)} kind="accuracy" />
                  </td>
                  <td className="model-actions-cell">
                    <div className="model-row-actions">
                      {row.active ? (
                        <StatusChip label="Active" tone="success" />
                      ) : row.selectable ? (
                        <button
                          className="secondary small"
                          onClick={(event) => {
                            event.stopPropagation();
                            void onActivateModel(row);
                          }}
                        >
                          Use
                        </button>
                      ) : null}
                      {row.supportsDownload && row.state !== "ready" && !row.selectable ? (
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
                      {row.supportsInstall ? (
                        <ActionButton
                          className="secondary small"
                          state={buttonFeedback[`model-link:${row.id}`]}
                          idleLabel={row.path ? "Choose another file" : "Use local file"}
                          workingLabel="Linking"
                          doneLabel="Linked"
                          idleIcon={<FolderIcon className="small-icon" />}
                          doneIcon={<CheckIcon className="small-icon" />}
                          onClick={() => onLinkCatalogModel(row)}
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
                      {!row.selectable && row.state === "ready" ? (
                        <StatusChip label="Unlocked" tone="accent" />
                      ) : null}
                      {row.hfUrl ? (
                        <button
                          className="secondary small icon-only-button"
                          onClick={(event) => {
                            event.stopPropagation();
                            void onOpenModelReference(row);
                          }}
                          aria-label="Open Hugging Face"
                          title="Open Hugging Face"
                        >
                          <ExternalIcon className="small-icon" />
                        </button>
                      ) : null}
                    </div>
                    <span className="model-row-footnote">
                      {row.state === "planned"
                        ? "Reference only"
                        : row.diskSizeBytes
                          ? formatBytes(row.diskSizeBytes)
                          : row.downloadSizeBytes
                            ? `~${formatBytes(row.downloadSizeBytes)}`
                            : hardwareFit.label}
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>

        {filteredModels.length === 0 ? <div className="empty-state">No models match.</div> : null}
      </div>

      {selectedModel ? (
        <article className="model-focus-card">
          <div className="surface-bar model-focus-head">
            <div className="surface-title">
              <span className="surface-title-label">{selectedModel.name}</span>
              <span className="model-focus-subtitle">
                {selectedModel.provider} · {selectedModel.speechMode} · {selectedModel.footprint}
              </span>
            </div>
            <div className="header-actions">
              <StatusChip
                label={
                  selectedModel.active
                    ? "Active"
                    : selectedModel.state === "ready"
                      ? selectedModel.selectable
                        ? "Ready"
                        : "Installed"
                      : selectedModel.selectable
                      ? "Ready"
                      : selectedModel.state === "downloadable"
                        ? "Downloadable"
                        : "Planned"
                }
                tone={
                  selectedModel.active
                    ? "success"
                    : selectedModel.state === "ready"
                      ? selectedModel.selectable
                        ? "success"
                        : "accent"
                      : selectedModel.selectable
                      ? "success"
                      : selectedModel.state === "downloadable"
                        ? "accent"
                        : "warning"
                }
              />
              {selectedModelFit ? (
                <StatusChip label={selectedModelFit.label} tone={selectedModelFit.tone} />
              ) : null}
            </div>
          </div>

          <div className="model-focus-layout">
            <div className="model-focus-copy">
              <div className="detail-copy model-focus-copy-block">
                <p>{selectedModel.summary}</p>
                <p>{selectedModel.note}</p>
                <p>{selectedModel.bestFor}</p>
                {selectedModelFit ? <p>{selectedModelFit.detail}</p> : null}
                {selectedModel.path ? <code className="path-chip">{selectedModel.path}</code> : null}
              </div>

              <div className="model-feature-list model-focus-capabilities">
                {selectedModel.featureBadges.map((feature) => (
                  <ModelFeatureBadge
                    key={`focus-${selectedModel.id}-${feature.id}`}
                    icon={featureIcon(feature.icon)}
                    label={feature.label}
                  />
                ))}
              </div>

              {selectedModel.unlockedFeatures?.length ? (
                <ul className="detail-list model-focus-unlocks">
                  {selectedModel.unlockedFeatures.map((feature) => (
                    <li key={feature}>{feature}</li>
                  ))}
                </ul>
              ) : null}

              <div className="inline-actions model-focus-actions">
                {selectedModel.supportsDownload ? (
                  <ActionButton
                    className="secondary"
                    state={buttonFeedback[`model-download:${selectedModel.id}`]}
                    idleLabel={
                      selectedModel.path
                        ? "Re-download from Hugging Face"
                        : "Download from Hugging Face"
                    }
                    workingLabel="Downloading"
                    doneLabel="Downloaded"
                    idleIcon={<DownloadIcon className="small-icon" />}
                    workingIcon={<DownloadIcon className="small-icon" />}
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={() => onDownloadCatalogModel(selectedModel)}
                  />
                ) : null}
                {selectedModel.supportsInstall ? (
                  <ActionButton
                    className="secondary"
                    state={buttonFeedback[`model-link:${selectedModel.id}`]}
                    idleLabel={selectedModel.path ? "Use another file" : "Use existing file"}
                    workingLabel="Linking"
                    doneLabel="Linked"
                    idleIcon={<FolderIcon className="small-icon" />}
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={() => onLinkCatalogModel(selectedModel)}
                  />
                ) : null}
                {selectedModel.managed ? (
                  <ActionButton
                    className="secondary"
                    state={buttonFeedback[`model-remove:${selectedModel.id}`]}
                    idleLabel="Delete downloaded model"
                    workingLabel="Deleting"
                    doneLabel="Deleted"
                    idleIcon={<TrashIcon className="small-icon" />}
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={() => onRemoveCatalogModel(selectedModel)}
                  />
                ) : null}
                {selectedModel.selectable && !selectedModel.active ? (
                  <button className="secondary" onClick={() => void onActivateModel(selectedModel)}>
                    Use model
                  </button>
                ) : null}
                {selectedModel.artifactUrl ? (
                  <button className="secondary" onClick={() => void onOpenModelArtifact(selectedModel)}>
                    Open compatible file
                  </button>
                ) : null}
                {selectedModel.hfUrl ? (
                  <button className="secondary" onClick={() => void onOpenModelReference(selectedModel)}>
                    Open Hugging Face
                  </button>
                ) : null}
              </div>

              <ul className="detail-list model-focus-highlights">
                {selectedModel.highlights.map((highlight) => (
                  <li key={highlight}>{highlight}</li>
                ))}
              </ul>
            </div>

            <div className="model-focus-metrics">
              <div className="model-focus-meta-list">
                {selectedModelMeta.map(([label, value]) => (
                  <div className="model-focus-meta-row" key={label}>
                    <span>{label}</span>
                    <strong>{value}</strong>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </article>
      ) : null}
    </section>
  );
}
