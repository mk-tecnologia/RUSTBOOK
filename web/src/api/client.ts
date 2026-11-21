const BASE_URL = ''

export class ApiError extends Error {
  public status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

export async function api<T = unknown>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const token = localStorage.getItem('token')
  const headers = new Headers(options.headers)
  headers.set('Content-Type', 'application/json')
  if (token) {
    headers.set('Authorization', `Bearer ${token}`)
  }

  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers,
  })

  if (!res.ok) {
    const body = await res.json().catch(() => ({})) as { error?: string }
    if (res.status === 401 && path !== '/api/login') {
      localStorage.removeItem('token')
      localStorage.removeItem('user')
      window.location.assign('/login')
    }
    throw new ApiError(res.status, body.error || res.statusText)
  }

  // Handle empty responses
  const text = await res.text()
  if (!text) return {} as T
  return JSON.parse(text)
}
