/** Audio capture hook — implemented in Sprint 1 Task 1.3 */
export function useAudioCapture() {
  return {
    isRecording: false,
    startRecording: async () => undefined,
    stopRecording: async () => undefined,
    cancelRecording: async () => undefined,
  };
}
