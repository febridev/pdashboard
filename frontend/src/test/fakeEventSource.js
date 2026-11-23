export class FakeEventSource {
  constructor(url) { this.url = url; this.listeners = {}; this.closed = false; FakeEventSource.instances.push(this) }
  addEventListener(type, fn) { (this.listeners[type] ??= []).push(fn) }
  close() { this.closed = true }
  dispatch(type, data) { for (const fn of this.listeners[type] ?? []) fn({ data }) }
}
FakeEventSource.instances = []
export function resetFakeEventSources() { FakeEventSource.instances.length = 0 }
