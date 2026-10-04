import {
  createContext,
  useContext,
  useLayoutEffect,
  useRef,
  useState,
  type PropsWithChildren,
} from "react";

const PageUiState = createContext<Map<string, unknown> | null>(null);

/** One page visit, including settings detours. Contains no timers or producers. */
export function PageUiStateProvider({ children }: PropsWithChildren) {
  const values = useRef(new Map<string, unknown>());
  return (
    <PageUiState.Provider value={values.current}>
      {children}
    </PageUiState.Provider>
  );
}

/** Retain only explicit UI choices (and the single paused process snapshot). */
export function usePageUiState<T>(key: string, initial: T | (() => T)) {
  const values = useContext(PageUiState);
  const state = useState<T>(() =>
    values?.has(key)
      ? (values.get(key) as T)
      : typeof initial === "function"
        ? (initial as () => T)()
        : initial,
  );
  useLayoutEffect(() => {
    values?.set(key, state[0]);
  }, [values, key, state[0]]);
  return state;
}
