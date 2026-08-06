export function localOnly() {
  return Promise.reject(new Error('Hydrogen Music 本地版不提供在线服务'))
}
