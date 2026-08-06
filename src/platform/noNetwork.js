function unavailable() {
  return Promise.reject(new Error('Hydrogen Music 本地版不提供网络请求'))
}

export default {
  get: unavailable,
  post: unavailable,
  request: unavailable,
}
