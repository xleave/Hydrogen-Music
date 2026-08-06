export default function request() {
  return Promise.reject(new Error('Hydrogen Music 本地版不提供网络请求'))
}
