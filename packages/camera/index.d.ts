export type CameraPermissionStatus =
  | "granted"
  | "denied"
  | "blocked"
  | "unknown";

type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
  | "location-disabled"
  | "timeout"
  | "protocol"
  | "unexpected";

interface ResourceError {
  readonly kind: ResourceErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

type AsyncResource<T> = { reload(): void } &
  (
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: ResourceError }
  );

export type CameraPermission = AsyncResource<CameraPermissionStatus> & {
  request(): void;
};

export type CameraErrorKind =
  | "permission-denied"
  | "permission-blocked"
  | "unavailable"
  | "busy"
  | "capture"
  | "storage"
  | "decoder"
  | "timeout"
  | "protocol"
  | "unexpected";

export interface CameraError {
  readonly kind: CameraErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

export type CameraSession<T> = { open(): void } &
  (
    | { readonly status: "idle" | "opening" | "active" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: CameraError }
  );

export interface CameraImageSource {
  readonly __inkCameraImageSource: never;
}

export interface CapturedPhoto {
  readonly source: CameraImageSource;
  readonly width: number;
  readonly height: number;
  readonly mimeType: "image/jpeg";
  readonly capturedAtMs: number;
}

export type CodeFormat =
  | "qr"
  | "aztec"
  | "data-matrix"
  | "pdf417"
  | "codabar"
  | "code-39"
  | "code-93"
  | "code-128"
  | "ean-8"
  | "ean-13"
  | "itf"
  | "upc-a"
  | "upc-e";

export interface CodeScannerOptions {
  readonly formats?: ReadonlyArray<CodeFormat>;
}

export interface CodeScan {
  readonly text: string;
  readonly format: CodeFormat;
}

export interface CameraPreviewProps {
  readonly session: CameraSession<CapturedPhoto> | CameraSession<CodeScan>;
}

/** Reserves an Ink-owned camera surface inside the current Screen. */
export declare function CameraPreview(props: CameraPreviewProps): object;

/** Reads and requests the camera permission shared by capture and scanning. */
export declare function cameraPermission(): CameraPermission;

/** Creates one screen-scoped photo capture session. */
export declare function photoCapture(): CameraSession<CapturedPhoto>;

/** Creates one screen-scoped code scanning session. */
export declare function codeScanner(
  options?: CodeScannerOptions,
): CameraSession<CodeScan>;
