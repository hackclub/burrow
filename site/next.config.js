/** @type {import('next').NextConfig} */
const nextConfig = {
  headers() {
    return [
      {
        source: "/.well-known/apple-app-site-association",
        headers: [{ key: "Content-Type", value: "application/json" }],
      }
    ];
  }
};

module.exports = nextConfig;
