import {
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";

export interface ModalDialogProps {
  children: ReactNode;
  className?: string;
  labelledBy: string;
  onClose(): void;
  returnFocusRef?: RefObject<HTMLElement | null>;
}

export function ModalDialog({
  children,
  className,
  labelledBy,
  onClose,
  returnFocusRef,
}: ModalDialogProps) {
  const dialog = useRef<HTMLDialogElement | null>(null);
  const [trigger] = useState<HTMLElement | null>(() =>
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  );

  useEffect(() => {
    const element = dialog.current;
    if (!element) {
      return;
    }
    const returnFocus = returnFocusRef?.current ?? trigger;

    try {
      element.showModal();
    } catch {
      element.setAttribute("open", "");
    }

    return () => {
      if (element.open && typeof element.close === "function") {
        element.close();
      }
      returnFocus?.focus();
    };
  }, [returnFocusRef, trigger]);

  return (
    <dialog
      ref={dialog}
      className={`profile-dialog ${className ?? ""}`}
      aria-labelledby={labelledBy}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
        }
      }}
    >
      {children}
    </dialog>
  );
}
