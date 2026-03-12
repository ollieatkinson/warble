import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = {
  children: ReactNode;
};

type State = {
  errorMessage: string | null;
};

export class AppErrorBoundary extends Component<Props, State> {
  state: State = {
    errorMessage: null,
  };

  static getDerivedStateFromError(error: unknown): State {
    return {
      errorMessage: error instanceof Error ? error.message : "Something went wrong.",
    };
  }

  componentDidCatch(error: unknown, errorInfo: ErrorInfo) {
    console.error("App render failed", error, errorInfo);
  }

  render() {
    if (this.state.errorMessage) {
      return (
        <main className="loading-shell loading-shell-error">
          <div className="loading-shell-copy">
            <strong>Warble hit a startup error.</strong>
            <span>{this.state.errorMessage}</span>
          </div>
        </main>
      );
    }

    return this.props.children;
  }
}
