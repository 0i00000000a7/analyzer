/// <reference types="vite/client" />

declare module '*?worker' {
  const workerConstructor: new () => Worker;
  export default workerConstructor;
}

declare module '*.css' {
  const content: string;
  export default content;
}
