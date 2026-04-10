import Head from "next/head";
import { Menu, Transition } from "@headlessui/react";
import { useState, useRef, useEffect } from "react";

function ChevronIcon({ open }: { open: boolean }) {
  return (
    <span aria-hidden="true" className="pl-1.5 text-lg">
      {open ? "▴" : "▾"}
    </span>
  );
}

function ExternalLinkIcon() {
  return (
    <span aria-hidden="true" className="pl-3 text-lg">
      ↗
    </span>
  );
}

function GithubIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      className="mr-3 inline-block h-4 w-4"
      fill="currentColor"
    >
      <path d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.5-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82A7.6 7.6 0 0 1 8 4.84c.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8" />
    </svg>
  );
}

export default function Page() {
  const [chevron, setChevron] = useState(false);
  const menuButtonRef = useRef<HTMLButtonElement | null>(null);
  const toggleDropdown = () => {
    setChevron(!chevron);
  };
  const handleClickOutside = (event: MouseEvent) => {
    if (
      menuButtonRef.current &&
      !menuButtonRef.current.contains(event.target as Node)
    ) {
      setChevron(false);
    }
  };
  useEffect(() => {
    document.addEventListener("click", handleClickOutside);

    return () => {
      document.removeEventListener("click", handleClickOutside);
    };
  }, []);
  return (
    <>
      <Head>
        <title>Burrow</title>
        <meta
          name="description"
          content="Burrow is an open-source tool for bypassing firewalls, built by teenagers at Hack Club."
        />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      </Head>
      <div className="flex min-h-screen items-center justify-center">
        <div className="mb-48 text-center">
          <h1 className="font-PhantomSans text-5xl md:text-6xl lg:text-7xl xl:text-8xl 2xl:text-9xl">
            <span className="font-bold text-hackClubRed">Burrow</span> Through{" "}
            Firewalls
          </h1>
          <div className="mx-auto my-2 w-11/12 font-PhantomSans text-lg md:my-10 md:w-3/4 lg:text-xl xl:text-2xl 2xl:text-3xl">
            <p>
              Burrow is an open source tool for burrowing through firewalls,
              built by teenagers at{" "}
              <span className="text-hackClubRed underline">
                <a href="https://www.hackclub.com/" target="_blank">
                  Hack Club.
                </a>
              </span>{" "}
              <span className="rounded-md bg-burrowHover p-0.5 font-SpaceMono text-hackClubBlue">
                burrow
              </span>{" "}
              is a Rust-based VPN for getting around restrictive Internet
              censors.
            </p>
          </div>
          <div className="flex flex-wrap justify-center">
            <div className="flex w-full justify-center gap-x-4 md:w-auto">
              <Menu as="div" className="relative inline-block text-left">
                <div>
                  <Menu.Button
                    onClick={() => toggleDropdown()}
                    ref={menuButtonRef}
                    className="w-50 h-12 rounded-2xl bg-hackClubRed px-3 font-SpaceMono hover:scale-105 md:h-12 md:w-auto md:rounded-3xl md:text-xl 2xl:h-16 2xl:text-2xl "
                  >
                    Install for Linux
                    <ChevronIcon open={chevron} />
                  </Menu.Button>
                </div>
                <Transition
                  enter="transition duration-100 ease-out"
                  enterFrom="transform scale-95 opacity-0"
                  enterTo="transform scale-100 opacity-100"
                  leave="transition duration-75 ease-out"
                  leaveFrom="transform scale-100 opacity-100"
                  leaveTo="transform scale-95 opacity-0"
                >
                  <Menu.Items
                    as="div"
                    className="absolute right-0 z-10 mt-2 h-auto w-auto origin-top-right  divide-black rounded-md bg-hackClubBlueShade font-SpaceMono text-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none md:text-xl 2xl:text-2xl"
                  >
                    <div className="divide-y-2 divide-hackClubRed px-1 py-1">
                      <Menu.Item>
                        {({ active }) => (
                          <a className="block px-4 py-2 hover:rounded-lg hover:bg-slate-600">
                            Install for Windows
                          </a>
                        )}
                      </Menu.Item>
                      <Menu.Item>
                        <a className="block px-4 py-2 hover:rounded-lg hover:bg-slate-600">
                          Install for MacOS
                        </a>
                      </Menu.Item>
                    </div>
                  </Menu.Items>
                </Transition>
              </Menu>
              <a>
                <button className="h-12 rounded-2xl border-2 border-hackClubRed bg-transparent px-3 font-SpaceMono text-lg text-hackClubRed hover:scale-110 md:h-12 md:rounded-3xl md:text-xl 2xl:h-16 2xl:text-2xl">
                  Docs
                  <ExternalLinkIcon />
                </button>
              </a>
            </div>
            <div className="mt-4 flex w-full justify-center hover:scale-110 md:mt-0 md:w-auto md:pl-4">
              <a href="https://github.com/hackclub/burrow" target="_blank">
                <button className="h-12 w-40 rounded-xl border-2 border-hackClubRed bg-transparent px-3 font-SpaceMono text-hackClubRed md:h-12 md:w-auto md:rounded-3xl md:text-xl 2xl:h-16 2xl:text-2xl">
                  <GithubIcon />
                  Contribute
                </button>
              </a>
            </div>
          </div>
          {/* Footer  */}
          {/* <div className="fixed bottom-0 mb-20 left-[25vw] md:left-[40vw] lg:left-[44vw]">
                        <a href="https://hackclub.com/" target="_blank">
                            <button className="flex items-center bg-transparent border-2 border-burrowStroke text-hackClubRed font-SpaceMono text-lg md:text-2xl rounded-xl md:rounded-2xl h-12 md:h-16 px-3">
                                <Image
                                    src="/hackclub.svg"
                                    width={35}
                                    height={35}
                                    className="mx-2"
                                    alt="Hack Club's logo"
                                />
                                By Hack Club
                            </button>
                        </a>
                    </div> */}
        </div>
      </div>
    </>
  );
}
